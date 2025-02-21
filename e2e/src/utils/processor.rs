use localic_std::modules::cosmwasm::{contract_execute, contract_query};
use localic_utils::utils::test_context::TestContext;
use log::info;
use valence_authorization_utils::authorization::Priority;
use valence_processor_utils::processor::MessageBatch;

/// queries the remote domain processor queue and tries to confirm that the queue length
/// matches `len`.
/// retries for 10 times with a 5 second sleep in between. fails after 10 retries.
pub fn confirm_remote_domain_processor_queue_length(
    test_ctx: &mut TestContext,
    processor_domain: &str,
    processor_addr: &str,
    len: usize,
) {
    let mut tries = 0;
    loop {
        let items =
            get_processor_queue_items(test_ctx, processor_domain, processor_addr, Priority::Medium);
        info!(
            "{processor_domain} processor queue (len {:?}): {:?}",
            items.len(),
            items
        );

        if items.len() == len {
            break;
        } else if tries > 10 {
            panic!("Batch not found after 10 tries");
        }

        tries += 1;
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

pub fn tick_processor(
    test_ctx: &mut TestContext,
    chain_name: &str,
    key: &str,
    processor_address: &str,
    flags: &str,
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
        flags,
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
