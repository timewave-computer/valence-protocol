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
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::Addr;

    // Use properly formatted valid bech32 addresses for testing
    const USER: &str = "neutron1m9l358xunhhwds0568za49mzhvuxx9ux8xafx2";
    const FACTORY_ADMIN: &str = "neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xp86p9fl";
    const CONTROLLER: &str = "neutron14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s4fgc8x";
    const LIBRARY1: &str = "neutron1zjccerddgmt0hwp2t2g9qe2y8hfrvl32a5ajd0";

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&Addr::unchecked(FACTORY_ADMIN), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create_account_basic() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&Addr::unchecked(FACTORY_ADMIN), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let request = AccountRequest {
            controller: CONTROLLER.to_string(),
            libraries: vec![LIBRARY1.to_string()],
            program_id: "42".to_string(),
            account_request_id: 123,
            historical_block_height: 1,
            signature: None,
        };
        let info = message_info(&Addr::unchecked(FACTORY_ADMIN), &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CreateAccount { request },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn test_create_account_empty_libraries() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&Addr::unchecked(FACTORY_ADMIN), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let request = AccountRequest {
            controller: CONTROLLER.to_string(),
            libraries: vec![],
            program_id: "42".to_string(),
            account_request_id: 124,
            historical_block_height: 1,
            signature: None,
        };
        let info = message_info(&Addr::unchecked(USER), &[]);
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CreateAccount { request },
        )
        .unwrap_err();
        assert!(err.to_string().contains("At least one library required"));
    }

    #[test]
    fn test_create_account_duplicate_nonce() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&Addr::unchecked(FACTORY_ADMIN), &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let request = AccountRequest {
            controller: CONTROLLER.to_string(),
            libraries: vec![LIBRARY1.to_string()],
            program_id: "42".to_string(),
            account_request_id: 125,
            historical_block_height: 1,
            signature: None,
        };

        // First creation should succeed
        let info = message_info(&Addr::unchecked(USER), &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::CreateAccount {
                request: request.clone(),
            },
        )
        .unwrap();

        // Second creation with same nonce should fail
        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::CreateAccount { request },
        )
        .unwrap_err();
        assert!(err.to_string().contains("already used"));
    }
}

#[cfg(test)]
mod integration_test;
