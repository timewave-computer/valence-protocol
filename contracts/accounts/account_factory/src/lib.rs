// Purpose: Account factory contract entry point and tests
pub mod contract;
pub mod msg;
pub mod state;

pub use contract::*;
pub use msg::*;
pub use state::*;

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Api, testing::{message_info, mock_dependencies, mock_env, MockApi}};

    // Use MockApi to create valid addresses for testing
    fn mock_api() -> MockApi {
        MockApi::default()
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let api = mock_api();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&api.addr_make("admin"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create_account_basic() {
        let mut deps = mock_dependencies();
        let api = mock_api();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&api.addr_make("admin"), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let env = mock_env();
        let request = AccountRequest {
            controller: api.addr_make("controller").to_string(),
            libraries: vec![api.addr_make("library1").to_string()],
            program_id: "42".to_string(),
            account_request_id: 123,
            historical_block_height: env.block.height - 10, // Use a past block height
            signature: None,
            public_key: None,
        };
        let info = message_info(&api.addr_make("admin"), &[]);
        let res = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::CreateAccount { request },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn test_create_account_empty_libraries() {
        let mut deps = mock_dependencies();
        let api = mock_api();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&api.addr_make("admin"), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let env = mock_env();
        let request = AccountRequest {
            controller: api.addr_make("controller").to_string(),
            libraries: vec![],
            program_id: "42".to_string(),
            account_request_id: 124,
            historical_block_height: env.block.height - 10, // Use a past block height
            signature: None,
            public_key: None,
        };
        let info = message_info(&api.addr_make("user"), &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CreateAccount { request },
        )
        .unwrap_err();
        assert!(err.to_string().contains("Libraries list cannot be empty"));
    }

    #[test]
    fn test_create_account_duplicate_nonce() {
        let mut deps = mock_dependencies();
        let api = mock_api();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&api.addr_make("admin"), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let env = mock_env();
        let request = AccountRequest {
            controller: api.addr_make("controller").to_string(),
            libraries: vec![api.addr_make("library1").to_string()],
            program_id: "42".to_string(),
            account_request_id: 125,
            historical_block_height: env.block.height - 10, // Use a past block height
            signature: None,
            public_key: None,
        };

        // First creation should succeed
        let info = message_info(&api.addr_make("user"), &[]);
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::CreateAccount {
                request: request.clone(),
            },
        )
        .unwrap();

        // Second creation with same nonce should fail
        let err = execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::CreateAccount { request },
        )
        .unwrap_err();
        assert!(err.to_string().contains("already used"));
    }

    #[test]
    fn test_secure_address_derivation() {
        let deps = mock_dependencies();
        let api = mock_api();
        
        // Test with a valid compressed secp256k1 public key (33 bytes, starts with 0x02 or 0x03)
        let valid_pubkey = [
            0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
            0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfe, 0xd9, 0x85, 0x30, 0x23, 0x97, 0xa0, 0xe0, 0xd1,
            0x15, 0x60, 0x7b, 0x4c, 0xf2,
        ];

        // This should succeed and create a proper Bech32 address
        let result = contract::execute::derive_address_from_pubkey(&deps.as_ref(), &valid_pubkey);
        assert!(result.is_ok());
        
        let address = result.unwrap();
        // Verify it's a valid address format (not manually constructed)
        assert!(api.addr_validate(&address.to_string()).is_ok());
        // Verify it's not the old insecure format (should not start with "cosmos1" followed by hex)
        let addr_str = address.to_string();
        assert!(
            !addr_str.starts_with("cosmos1")
                || !addr_str.chars().skip(7).all(|c| c.is_ascii_hexdigit())
        );

        // Test with invalid public key length
        let invalid_pubkey = [0x02; 32]; // Wrong length
        let result = contract::execute::derive_address_from_pubkey(&deps.as_ref(), &invalid_pubkey);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid public key length"));

        // Test with invalid public key format (doesn't start with 0x02 or 0x03)
        let invalid_format_pubkey = [0x04; 33]; // Invalid prefix
        let result =
            contract::execute::derive_address_from_pubkey(&deps.as_ref(), &invalid_format_pubkey);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid compressed secp256k1 public key format"));
    }
}

#[cfg(test)]
mod integration_test;
