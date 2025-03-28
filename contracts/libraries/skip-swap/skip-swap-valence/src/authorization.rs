use cosmwasm_std::{Addr, Decimal, Deps, StdResult};
use cw_utils::Expiration;
use valence_authorization_utils::{
    authorization::{
        AtomicSubroutine, AuthorizationDuration, AuthorizationInfo as ValAuthInfo, AuthorizationModeInfo, 
        PermissionTypeInfo, Priority, Subroutine,
    },
    authorization_message::{Message, MessageDetails, MessageType},
    domain::Domain,
    function::AtomicFunction,
};
use valence_library_utils::LibraryAccountType;

// Re-export the AuthorizationInfo for use within the crate
pub use valence_authorization_utils::authorization::AuthorizationInfo;

/// Response for authorization queries
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
pub struct IsAuthorizedResponse {
    pub is_authorized: bool,
}

/// Create an authorization for swap operations
pub fn create_swap_authorization(
    strategist_address: &Addr,
    _swap_routes: Vec<(String, String)>, // (input_token, output_token) pairs
    _venues: Vec<String>,
    _max_slippage: Decimal,
) -> ValAuthInfo {
    // Create the message details for the swap operation
    let message_details = MessageDetails {
        message_type: MessageType::CosmwasmExecuteMsg,
        message: Message {
            name: "execute_swap".to_string(),
            params_restrictions: None,
        },
    };

    // Create the atomic function for the swap
    let function = AtomicFunction {
        domain: Domain::Main,
        message_details,
        contract_address: LibraryAccountType::Addr(strategist_address.to_string()),
    };

    // Create the subroutine with the function
    let subroutine = Subroutine::Atomic(AtomicSubroutine {
        functions: vec![function],
        retry_logic: None,
        expiration_time: None,
    });

    // Create the authorization info
    ValAuthInfo {
        label: "skip_swap".to_string(),
        mode: AuthorizationModeInfo::Permissioned(PermissionTypeInfo::WithoutCallLimit(vec![strategist_address.to_string()])),
        not_before: Expiration::AtHeight(0),
        duration: AuthorizationDuration::Forever,
        max_concurrent_executions: Some(1),
        subroutine,
        priority: Some(Priority::Medium),
    }
}

/// Checks if a strategist is authorized
/// 
/// This is a local validation - integration with the authorization contract
/// would require sending actual messages through the authorization contract.
pub fn is_strategist_authorized(
    _deps: Deps,
    config_strategist: &Addr,
    strategist_address: &Addr,
) -> StdResult<IsAuthorizedResponse> {
    Ok(IsAuthorizedResponse {
        is_authorized: config_strategist == strategist_address,
    })
}

/// Checks if an asset pair is authorized
/// 
/// This function first checks against the local config, then can be extended
/// to check with the Valence authorization contract in the future.
pub fn is_asset_pair_authorized(
    _deps: Deps,
    allowed_pairs: &[crate::types::AssetPair],
    input_asset: &str,
    output_asset: &str,
) -> StdResult<IsAuthorizedResponse> {
    let is_authorized = allowed_pairs.iter().any(|pair| {
        pair.input_asset == input_asset && pair.output_asset == output_asset
    });
    
    Ok(IsAuthorizedResponse { is_authorized })
}

/// Checks if a swap venue is authorized
/// 
/// This function first checks against the local config, then can be extended
/// to check with the Valence authorization contract in the future.
pub fn is_swap_venue_authorized(
    _deps: Deps,
    allowed_venues: &[String],
    venue: &str,
) -> StdResult<IsAuthorizedResponse> {
    Ok(IsAuthorizedResponse {
        is_authorized: allowed_venues.contains(&venue.to_string()),
    })
}

/// Checks if a slippage amount is authorized
/// 
/// This function first checks against the local config, then can be extended
/// to check with the Valence authorization contract in the future.
pub fn is_slippage_authorized(
    _deps: Deps,
    max_slippage: Decimal,
    slippage: Decimal,
) -> StdResult<IsAuthorizedResponse> {
    Ok(IsAuthorizedResponse {
        is_authorized: slippage <= max_slippage,
    })
}

/// SwapMessage contains the information needed to execute a swap on the Skip contract
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
pub struct SwapMessage {
    pub contract_addr: String,
    pub input_denom: String,
    pub input_amount: String,
    pub output_denom: String,
    pub min_output_amount: String,
    pub slippage: String,
    pub venue: String,
}

/// Create a processor message for a skip swap
pub fn create_swap_message(
    skip_contract: &Addr,
    input_denom: &str,
    input_amount: u128,
    output_denom: &str,
    min_output_amount: u128,
    slippage: Decimal,
    venue: &str,
) -> SwapMessage {
    // Create a SwapMessage structure with the swap parameters
    SwapMessage {
        contract_addr: skip_contract.to_string(),
        input_denom: input_denom.to_string(),
        input_amount: input_amount.to_string(),
        output_denom: output_denom.to_string(),
        min_output_amount: min_output_amount.to_string(),
        slippage: slippage.to_string(),
        venue: venue.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_strategist_authorization() {
        let deps = mock_dependencies();
        let strategist = Addr::unchecked("strategist");
        
        // Test valid strategist
        let result = is_strategist_authorized(
            deps.as_ref(), 
            &strategist, 
            &Addr::unchecked("strategist")
        ).unwrap();
        assert!(result.is_authorized);
        
        // Test invalid strategist
        let result = is_strategist_authorized(
            deps.as_ref(), 
            &strategist, 
            &Addr::unchecked("not_strategist")
        ).unwrap();
        assert!(!result.is_authorized);
    }
    
    #[test]
    fn test_asset_pair_authorization() {
        let deps = mock_dependencies();
        let allowed_pairs = vec![
            crate::types::AssetPair {
                input_asset: "uusdc".to_string(),
                output_asset: "steth".to_string(),
            }
        ];
        
        // Test valid pair
        let result = is_asset_pair_authorized(
            deps.as_ref(),
            &allowed_pairs,
            "uusdc",
            "steth"
        ).unwrap();
        assert!(result.is_authorized);
        
        // Test invalid pair
        let result = is_asset_pair_authorized(
            deps.as_ref(),
            &allowed_pairs,
            "invalid",
            "steth"
        ).unwrap();
        assert!(!result.is_authorized);
    }
    
    #[test]
    fn test_venue_authorization() {
        let deps = mock_dependencies();
        let allowed_venues = vec!["astroport".to_string()];
        
        // Test valid venue
        let result = is_swap_venue_authorized(
            deps.as_ref(),
            &allowed_venues,
            "astroport"
        ).unwrap();
        assert!(result.is_authorized);
        
        // Test invalid venue
        let result = is_swap_venue_authorized(
            deps.as_ref(),
            &allowed_venues,
            "invalid"
        ).unwrap();
        assert!(!result.is_authorized);
    }
    
    #[test]
    fn test_slippage_authorization() {
        let deps = mock_dependencies();
        let max_slippage = Decimal::percent(1);
        
        // Test valid slippage
        let result = is_slippage_authorized(
            deps.as_ref(),
            max_slippage,
            Decimal::percent(1)
        ).unwrap();
        assert!(result.is_authorized);
        
        // Test invalid slippage
        let result = is_slippage_authorized(
            deps.as_ref(),
            max_slippage,
            Decimal::percent(2)
        ).unwrap();
        assert!(!result.is_authorized);
    }
    
    #[test]
    fn test_create_swap_authorization() {
        let strategist = Addr::unchecked("strategist");
        let routes = vec![("uusdc".to_string(), "steth".to_string())];
        let venues = vec!["astroport".to_string()];
        let max_slippage = Decimal::percent(1);
        
        let auth = create_swap_authorization(&strategist, routes.clone(), venues.clone(), max_slippage);
        
        assert_eq!(auth.label, "skip_swap");
        assert!(matches!(auth.mode, AuthorizationModeInfo::Permissioned(_)));
        assert!(matches!(auth.duration, AuthorizationDuration::Forever));
        assert_eq!(auth.max_concurrent_executions, Some(1));
    }
    
    #[test]
    fn test_create_swap_message() {
        let skip_contract = Addr::unchecked("skip_contract");
        let message = create_swap_message(
            &skip_contract,
            "uusd",
            1000000,
            "uatom",
            950000,
            Decimal::percent(5),
            "astroport"
        );
        
        assert_eq!(message.contract_addr, "skip_contract");
        assert_eq!(message.input_denom, "uusd");
        assert_eq!(message.input_amount, "1000000");
        assert_eq!(message.output_denom, "uatom");
        assert_eq!(message.min_output_amount, "950000");
        assert_eq!(message.slippage, "0.05");
        assert_eq!(message.venue, "astroport");
    }
} 