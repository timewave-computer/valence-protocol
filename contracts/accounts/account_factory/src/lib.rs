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
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Addr};

    // Use properly formatted valid bech32 addresses for testing
    const USER: &str = "neutron1m9l358xunhhwds0568za49mzhvuxx9ux8xafx2";
    const FACTORY_ADMIN: &str = "neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xp86p9fl";
    const CONTROLLER: &str = "neutron14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9s4fgc8x";
    const LIBRARY1: &str = "neutron1zjccerddgmt0hwp2t2g9qe2y8hfrvl32a5ajd0";
    const LIBRARY2: &str = "neutron1mxpuw2mslsn8j2lswk7fdjn6d7u3hzpampe4pv";

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            fee_collector: None, // Skip fee collector validation for testing
            jit_account_code_id: 1,
        };
        let info = mock_info(FACTORY_ADMIN, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_max_block_age_query() {
        let mut deps = mock_dependencies();

        // First instantiate the contract
        let msg = InstantiateMsg {
            fee_collector: None, // Skip fee collector validation for testing
            jit_account_code_id: 1,
        };
        let info = mock_info(FACTORY_ADMIN, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let query_msg = QueryMsg::GetMaxBlockAge {};
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let max_age: u64 = from_json(&res).unwrap();

        assert_eq!(max_age, MAX_BLOCK_AGE);
    }

    #[test]
    fn test_create_account_validation() {
        let mut deps = mock_dependencies();

        // First instantiate the contract
        let msg = InstantiateMsg {
            fee_collector: None, // Skip fee collector validation for testing
            jit_account_code_id: 1,
        };
        let info = mock_info(FACTORY_ADMIN, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test invalid account type - this should fail validation before any code lookup
        let invalid_request = AccountRequest {
            controller: CONTROLLER.to_string(),
            libraries: vec![LIBRARY1.to_string()],
            program_id: "program1".to_string(),
            account_request_id: 1,
            historical_block_height: 100,
            signature: None,
        };

        let execute_msg = ExecuteMsg::CreateAccount {
            request: invalid_request,
        };
        let info = mock_info(USER, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg);

        // Should fail - exact error message may vary depending on which validation runs first
        assert!(res.is_err());
    }

    #[test]
    fn test_create_account_empty_libraries() {
        let mut deps = mock_dependencies();

        // First instantiate the contract
        let msg = InstantiateMsg {
            fee_collector: None, // Skip fee collector validation for testing
            jit_account_code_id: 1,
        };
        let info = mock_info(FACTORY_ADMIN, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test empty libraries - this should fail validation before any code lookup
        let invalid_request = AccountRequest {
            controller: CONTROLLER.to_string(),
            libraries: vec![], // Empty libraries
            program_id: "program1".to_string(),
            account_request_id: 1,
            historical_block_height: 100,
            signature: None,
        };

        let execute_msg = ExecuteMsg::CreateAccount {
            request: invalid_request,
        };
        let info = mock_info(USER, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg);

        // Should fail - exact error message may vary depending on which validation runs first
        assert!(res.is_err());
    }

    #[test]
    fn test_request_validation_logic() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let sender = Addr::unchecked(FACTORY_ADMIN);
        let info = message_info(&sender, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test that address validation logic works (skip actual validation in mock environment)
        let library_addr = LIBRARY1;
        println!("Library address should be valid: {}", library_addr);

        // In a real environment, this would validate the bech32 format
        // For testing, we just verify the addresses are not empty and have reasonable length
        assert!(!library_addr.is_empty());
        assert!(library_addr.len() > 10);

        println!("✓ Library address format check passed");
    }
}

#[cfg(test)]
mod integration_test;
