use localic_std::modules::cosmwasm::contract_execute;
use localic_utils::utils::test_context::TestContext;
use log::info;

use super::GAS_FLAGS;

/// Tick a processor
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
