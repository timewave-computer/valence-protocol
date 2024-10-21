use localic_std::modules::cosmwasm::{contract_execute, contract_query};
use localic_utils::utils::test_context::TestContext;
use log::info;
use valence_authorization_utils::authorization::Priority;
use valence_processor_utils::processor::MessageBatch;

use super::GAS_FLAGS;

pub fn tick_processor(
    test_ctx: &mut TestContext,
    chain_name: &str,
    key: &str,
    processor_address: &str,
) {
    info!("Ticking processor on {}...", chain_name);
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(chain_name),
        processor_address,
        key,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(3));
}

pub fn get_processor_queue_items(
    test_ctx: &mut TestContext,
    chain_name: &str,
    processor_address: &str,
    priority: Priority,
) -> Vec<MessageBatch> {
    serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(chain_name),
            processor_address,
            &serde_json::to_string(&valence_processor_utils::msg::QueryMsg::GetQueue {
                from: None,
                to: None,
                priority,
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap()
}
