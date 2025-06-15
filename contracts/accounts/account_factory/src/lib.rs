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
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};

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
}

#[cfg(test)]
mod integration_test;
