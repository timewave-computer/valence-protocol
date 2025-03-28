use cosmwasm_std::Addr;

use crate::error::ContractError;
use crate::types::{Config, SkipRouteResponse};

/// Validates that the strategist is authorized
pub fn validate_strategist(
    config: &Config,
    strategist_address: &Addr,
) -> Result<(), ContractError> {
    if &config.strategist_address != strategist_address {
        return Err(ContractError::UnauthorizedStrategist {
            address: strategist_address.to_string(),
        });
    }
    Ok(())
}

/// Validates that the asset pair is allowed
pub fn validate_asset_pair(
    config: &Config,
    input_asset: &str,
    output_asset: &str,
) -> Result<(), ContractError> {
    let is_valid = config.allowed_asset_pairs.iter().any(|pair| {
        pair.input_asset == input_asset && pair.output_asset == output_asset
    });

    if !is_valid {
        return Err(ContractError::InvalidAssetPair {
            input_asset: input_asset.to_string(),
            output_asset: output_asset.to_string(),
        });
    }
    Ok(())
}

/// Validates that all venues in the route are allowed
pub fn validate_venues(
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

/// Validates a complete optimized route
pub fn validate_optimized_route(
    config: &Config,
    strategist_address: &Addr,
    input_asset: &str,
    output_asset: &str,
    route: &SkipRouteResponse,
) -> Result<(), ContractError> {
    validate_strategist(config, strategist_address)?;
    validate_asset_pair(config, input_asset, output_asset)?;
    validate_venues(config, route)?;
    validate_slippage(config, route)?;
    validate_destination(config, output_asset)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Decimal, Uint128};
    use std::collections::HashMap;
    use crate::types::{AssetPair, SwapDetails, SwapOperation};

    // Helper function to create a test configuration
    fn create_test_config() -> Config {
        let mut token_destinations = HashMap::new();
        token_destinations.insert("steth".to_string(), Addr::unchecked("dest_account"));

        let mut intermediate_accounts = HashMap::new();
        intermediate_accounts.insert("uusdc".to_string(), Addr::unchecked("intermediate_account"));

        Config {
            strategist_address: Addr::unchecked("strategist"),
            skip_entry_point: Addr::unchecked("skip_entry"),
            allowed_asset_pairs: vec![
                AssetPair {
                    input_asset: "uusdc".to_string(),
                    output_asset: "steth".to_string(),
                },
                AssetPair {
                    input_asset: "uatom".to_string(),
                    output_asset: "steth".to_string(),
                },
            ],
            allowed_venues: vec!["astroport".to_string(), "osmosis".to_string()],
            max_slippage: Decimal::percent(1),
            token_destinations,
            intermediate_accounts,
        }
    }

    // Helper function to create a test route
    fn create_test_route() -> SkipRouteResponse {
        SkipRouteResponse {
            source_chain_id: "neutron".to_string(),
            source_asset_denom: "uusdc".to_string(),
            dest_chain_id: "neutron".to_string(),
            dest_asset_denom: "steth".to_string(),
            amount: Uint128::new(1000000),
            operations: vec![SwapOperation {
                chain_id: "neutron".to_string(),
                operation_type: "swap".to_string(),
                swap_venue: Some("astroport".to_string()),
                swap_details: Some(SwapDetails {
                    input_denom: "uusdc".to_string(),
                    output_denom: "steth".to_string(),
                    pool_id: Some("pool1".to_string()),
                }),
                transfer_details: None,
            }],
            expected_output: Uint128::new(990000),
            slippage_tolerance_percent: Decimal::percent(1),
        }
    }

    #[test]
    fn test_validate_strategist() {
        let config = create_test_config();

        // Valid strategist
        let result = validate_strategist(&config, &Addr::unchecked("strategist"));
        assert!(result.is_ok());

        // Invalid strategist
        let result = validate_strategist(&config, &Addr::unchecked("not_strategist"));
        assert!(matches!(
            result,
            Err(ContractError::UnauthorizedStrategist { .. })
        ));
    }

    #[test]
    fn test_validate_asset_pair() {
        let config = create_test_config();

        // Valid asset pair
        let result = validate_asset_pair(&config, "uusdc", "steth");
        assert!(result.is_ok());

        // Invalid asset pair
        let result = validate_asset_pair(&config, "invalid", "steth");
        assert!(matches!(
            result,
            Err(ContractError::InvalidAssetPair { .. })
        ));
    }

    #[test]
    fn test_validate_venues() {
        let config = create_test_config();
        let mut route = create_test_route();

        // Valid venue
        let result = validate_venues(&config, &route);
        assert!(result.is_ok());

        // Invalid venue
        route.operations[0].swap_venue = Some("invalid".to_string());
        let result = validate_venues(&config, &route);
        assert!(matches!(result, Err(ContractError::InvalidVenue { .. })));
    }

    #[test]
    fn test_validate_slippage() {
        let config = create_test_config();
        let mut route = create_test_route();

        // Valid slippage
        let result = validate_slippage(&config, &route);
        assert!(result.is_ok());

        // Invalid slippage
        route.slippage_tolerance_percent = Decimal::percent(2);
        let result = validate_slippage(&config, &route);
        assert!(matches!(
            result,
            Err(ContractError::ExcessiveSlippage { .. })
        ));
    }

    #[test]
    fn test_validate_destination() {
        let config = create_test_config();

        // Valid destination
        let result = validate_destination(&config, "steth");
        assert!(result.is_ok());

        // Invalid destination
        let result = validate_destination(&config, "invalid");
        assert!(matches!(
            result,
            Err(ContractError::MissingDestination { .. })
        ));
    }

    #[test]
    fn test_validate_optimized_route() {
        let config = create_test_config();
        let route = create_test_route();

        // All validations pass
        let result = validate_optimized_route(
            &config,
            &Addr::unchecked("strategist"),
            "uusdc",
            "steth",
            &route,
        );
        assert!(result.is_ok());

        // One validation fails (invalid strategist)
        let result = validate_optimized_route(
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
} 