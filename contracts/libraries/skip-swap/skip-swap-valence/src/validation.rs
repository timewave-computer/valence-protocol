/*
 * Validation module for the Skip Swap Valence contract.
 * Provides validation logic for swap operations and routes:
 * - Validating optimized routes from the strategist
 * - Creating and verifying swap authorizations
 * - Checking parameter boundaries (slippage, timeout, etc.)
 * - Validating asset pairs against allowed configurations
 */

use cosmwasm_std::{Addr, Deps};

use crate::authorization::{
    is_asset_pair_authorized, is_strategist_authorized,
    create_swap_authorization
};
use crate::error::ContractError;
use crate::types::{Config, SkipRouteResponse};

/// Validates that the strategist is authorized
pub fn validate_strategist(
    deps: Deps,
    config: &Config,
    strategist_address: &Addr,
) -> Result<(), ContractError> {
    // Check if authorization contract should be used
    let auth_contract = match &config.authorization_contract {
        Some(addr) if config.use_authorization_contract => addr,
        _ => {
            // If no authorization contract or not using it, do a local check
            if &config.strategist_address != strategist_address {
                return Err(ContractError::UnauthorizedStrategist {
                    address: strategist_address.to_string(),
                });
            }
            return Ok(());
        }
    };
    
    // Use the authorization contract for validation
    let response = is_strategist_authorized(
        deps, 
        &config.strategist_address, 
        strategist_address,
        auth_contract
    )?;
    
    if !response.is_authorized {
        return Err(ContractError::UnauthorizedStrategist {
            address: strategist_address.to_string(),
        });
    }
    
    Ok(())
}

/// Validates that the asset pair is allowed
pub fn validate_asset_pair(
    deps: Deps,
    config: &Config,
    input_asset: &str,
    output_asset: &str,
) -> Result<(), ContractError> {
    // Get the auth contract if available and enabled
    let auth_contract = if config.use_authorization_contract {
        config.authorization_contract.as_ref()
    } else {
        None
    };
    
    let response = is_asset_pair_authorized(
        deps, 
        &config.allowed_asset_pairs, 
        input_asset, 
        output_asset,
        auth_contract
    )?;
    
    if !response.is_authorized {
        return Err(ContractError::InvalidAssetPair {
            input_asset: input_asset.to_string(),
            output_asset: output_asset.to_string(),
        });
    }
    
    Ok(())
}

/// Validates that all venues in the route are allowed
pub fn validate_venues(
    _deps: Deps,
    config: &Config,
    route: &SkipRouteResponse,
) -> Result<(), ContractError> {
    for operation in &route.operations {
        if let Some(venue) = &operation.swap_venue {
            if !config.allowed_venues.contains(venue) {
                return Err(ContractError::InvalidVenue {
                    venue: venue.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Validates that the slippage is within the maximum allowed
pub fn validate_slippage(
    _deps: Deps,
    config: &Config,
    route: &SkipRouteResponse,
) -> Result<(), ContractError> {
    if route.slippage_tolerance_percent > config.max_slippage {
        return Err(ContractError::ExcessiveSlippage {
            slippage: route.slippage_tolerance_percent,
            max_slippage: config.max_slippage,
        });
    }
    
    Ok(())
}

/// Validates that there's a destination for the output token
pub fn validate_destination(
    _deps: Deps,
    config: &Config,
    output_asset: &str,
) -> Result<(), ContractError> {
    if !config.token_destinations.contains_key(output_asset) {
        return Err(ContractError::MissingDestination {
            token: output_asset.to_string(),
        });
    }
    Ok(())
}

/// Creates a Valence authorization for skip swap operations based on the configuration
pub fn create_skip_swap_authorization(config: &Config) -> Result<(), ContractError> {
    // Extract the asset pairs from the config
    let asset_pairs: Vec<(String, String)> = config.allowed_asset_pairs
        .iter()
        .map(|pair| (pair.input_asset.clone(), pair.output_asset.clone()))
        .collect();
    
    // Create the authorization
    let _auth_info = create_swap_authorization(
        &config.strategist_address,
        asset_pairs,
        config.allowed_venues.clone(),
        config.max_slippage,
    );
    
    // In a real implementation, we would send this authorization to the authorization contract
    // This function would need to be modified to return a CosmosMsg to do that
    
    Ok(())
}

/// Validates a complete optimized route
pub fn validate_optimized_route(
    deps: Deps,
    config: &Config,
    strategist_address: &Addr,
    input_asset: &str,
    output_asset: &str,
    route: &SkipRouteResponse,
) -> Result<(), ContractError> {
    validate_strategist(deps, config, strategist_address)?;
    validate_asset_pair(deps, config, input_asset, output_asset)?;
    validate_venues(deps, config, route)?;
    validate_slippage(deps, config, route)?;
    validate_destination(deps, config, output_asset)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Decimal, Uint128, testing::mock_dependencies};
    use std::collections::HashMap;

    #[test]
    fn test_validations() {
        // Create mock dependencies
        let deps = mock_dependencies();
        
        // Create a config for testing
        let mut config = Config {
            owner: Addr::unchecked("owner"),
            strategist_address: Addr::unchecked("strategist"),
            skip_entry_point: Addr::unchecked("skip_entry"),
            allowed_asset_pairs: vec![
                crate::types::AssetPair {
                    input_asset: "uusdc".to_string(),
                    output_asset: "steth".to_string(),
                },
            ],
            allowed_venues: vec!["astroport".to_string()],
            max_slippage: Decimal::percent(1),
            token_destinations: HashMap::new(),
            intermediate_accounts: HashMap::new(),
            authorization_contract: None,
            use_authorization_contract: false,
            swap_authorization_label: "skip_swap".to_string(),
        };

        // Add a destination for steth
        config.token_destinations.insert("steth".to_string(), Addr::unchecked("destination"));

        // Create a route for testing
        let mut route = SkipRouteResponse {
            source_chain_id: "neutron".to_string(),
            source_asset_denom: "uusdc".to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: "steth".to_string(),
            amount: Uint128::new(1000000),
            operations: vec![
                crate::types::SwapOperation {
                    chain_id: "neutron".to_string(),
                    operation_type: "swap".to_string(),
                    swap_venue: Some("astroport".to_string()),
                    swap_details: Some(crate::types::SwapDetails {
                        input_denom: "uusdc".to_string(),
                        output_denom: "steth".to_string(),
                        pool_id: None,
                    }),
                    transfer_details: None,
                },
            ],
            expected_output: Uint128::new(500000),
            slippage_tolerance_percent: Decimal::percent(1),
        };

        // Test strategist validation
        {
            // Valid strategist
            let result = validate_strategist(deps.as_ref(), &config, &Addr::unchecked("strategist"));
            assert!(result.is_ok());

            // Invalid strategist
            let result = validate_strategist(deps.as_ref(), &config, &Addr::unchecked("not_strategist"));
            assert!(matches!(
                result,
                Err(ContractError::UnauthorizedStrategist { .. })
            ));
        }

        // Test asset pair validation
        {
            // Valid asset pair
            let result = validate_asset_pair(deps.as_ref(), &config, "uusdc", "steth");
            assert!(result.is_ok());

            // Invalid asset pair
            let result = validate_asset_pair(deps.as_ref(), &config, "invalid", "steth");
            assert!(matches!(
                result,
                Err(ContractError::InvalidAssetPair { .. })
            ));
        }

        // Test venue validation
        {
            // Valid venue
            let result = validate_venues(deps.as_ref(), &config, &route);
            assert!(result.is_ok());

            // Invalid venue
            route.operations[0].swap_venue = Some("invalid".to_string());
            let result = validate_venues(deps.as_ref(), &config, &route);
            assert!(matches!(result, Err(ContractError::InvalidVenue { .. })));
        }

        // Reset venue for other tests
        route.operations[0].swap_venue = Some("astroport".to_string());

        // Test slippage validation
        {
            // Valid slippage
            let result = validate_slippage(deps.as_ref(), &config, &route);
            assert!(result.is_ok());

            // Invalid slippage
            route.slippage_tolerance_percent = Decimal::percent(2);
            let result = validate_slippage(deps.as_ref(), &config, &route);
            assert!(matches!(
                result,
                Err(ContractError::ExcessiveSlippage { .. })
            ));
        }

        // Reset slippage for other tests
        route.slippage_tolerance_percent = Decimal::percent(1);

        // Test destination validation
        {
            // Valid destination
            let result = validate_destination(deps.as_ref(), &config, "steth");
            assert!(result.is_ok());

            // Invalid destination
            let result = validate_destination(deps.as_ref(), &config, "invalid");
            assert!(matches!(
                result,
                Err(ContractError::MissingDestination { .. })
            ));
        }

        // Test complete route validation
        {
            // All validations pass
            let result = validate_optimized_route(
                deps.as_ref(),
                &config,
                &Addr::unchecked("strategist"),
                "uusdc",
                "steth",
                &route,
            );
            assert!(result.is_ok());

            // One validation fails (invalid strategist)
            let result = validate_optimized_route(
                deps.as_ref(),
                &config,
                &Addr::unchecked("not_strategist"),
                "uusdc",
                "steth",
                &route,
            );
            assert!(matches!(
                result,
                Err(ContractError::UnauthorizedStrategist { .. })
            ));
        }
        
        // Test creating a Valence authorization
        let auth_result = create_skip_swap_authorization(&config);
        assert!(auth_result.is_ok());
    }
} 