// Purpose: Integration tests for account factory contract
use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::Addr;

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_basic_integration() {
        let mut deps = mock_dependencies();

        // Just test that we can instantiate the contract
        let msg = InstantiateMsg {
            fee_collector: None,
            jit_account_code_id: 1,
        };
        let info = message_info(&Addr::unchecked("admin"), &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
