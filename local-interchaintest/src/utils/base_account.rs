use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate};
use localic_utils::utils::test_context::TestContext;
use log::info;

use crate::utils::GAS_FLAGS;

/// Creates valence base accounts on a specific chain for our libraries and returns their contract address
pub fn create_base_accounts(
    test_ctx: &mut TestContext,
    key: &str,
    chain_name: &str,
    code_id: u64,
    admin: String,
    approved_libraries: Vec<String>,
    num_accounts: u64,
) -> Vec<String> {
    info!(
        "Creating {} base accounts on {}...",
        num_accounts, chain_name
    );
    let instantiate_msg = valence_account_utils::msg::InstantiateMsg {
        admin,
        approved_libraries,
    };
    let mut accounts = Vec::new();
    for _ in 0..num_accounts {
        let contract = contract_instantiate(
            test_ctx
                .get_request_builder()
                .get_request_builder(chain_name),
            key,
            code_id,
            &serde_json::to_string(&instantiate_msg).unwrap(),
            "valence_base_account",
            None,
            "",
        )
        .unwrap();

        accounts.push(contract.address);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    accounts
}

/// Approve a library for a base account
pub fn approve_library(
    test_ctx: &mut TestContext,
    chain_name: &str,
    key: &str,
    base_account: &str,
    library: String,
) {
    let approve_msg = valence_account_utils::msg::ExecuteMsg::ApproveLibrary {
        library: library.clone(),
    };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(chain_name),
        base_account,
        key,
        &serde_json::to_string(&approve_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!(
        "Approved library {} for base account {}",
        library, base_account
    );
    std::thread::sleep(std::time::Duration::from_secs(2));
}
