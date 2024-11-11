use cosmwasm_std::Coin;
use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate};
use localic_utils::utils::test_context::TestContext;
use log::info;

use crate::utils::GAS_FLAGS;

#[allow(clippy::too_many_arguments)]
/// Creates valence base accounts on a specific chain for our services and returns their contract address
pub fn create_base_accounts(
    test_ctx: &mut TestContext,
    key: &str,
    chain_name: &str,
    code_id: u64,
    admin: String,
    approved_services: Vec<String>,
    num_accounts: u64,
    fees: Option<Coin>,
) -> Vec<String> {
    info!(
        "Creating {} base accounts on {}...",
        num_accounts, chain_name
    );
    let instantiate_msg = valence_account_utils::msg::InstantiateMsg {
        admin,
        approved_services,
    };
    let flags = if let Some(fees) = fees {
        format!("--fees {}{}", fees.amount, fees.denom)
    } else {
        "".to_string()
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
            &flags,
        )
        .unwrap();

        accounts.push(contract.address);
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    accounts
}

/// Approve a service for a base account
pub fn approve_service(
    test_ctx: &mut TestContext,
    chain_name: &str,
    key: &str,
    base_account: &str,
    service: String,
    flags: Option<String>,
) {
    let approve_msg = valence_account_utils::msg::ExecuteMsg::ApproveService {
        service: service.clone(),
    };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(chain_name),
        base_account,
        key,
        &serde_json::to_string(&approve_msg).unwrap(),
        &format!(
            "{}{}",
            GAS_FLAGS,
            flags
                .map(|mut s| {
                    if !s.starts_with(" ") {
                        s.insert(0, ' ');
                    }
                    s
                })
                .unwrap_or_default()
        ),
    )
    .unwrap();

    info!(
        "Approved service {} for base account {}",
        service, base_account
    );
    std::thread::sleep(std::time::Duration::from_secs(2));
}
