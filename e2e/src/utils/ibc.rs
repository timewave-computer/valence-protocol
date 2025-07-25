use std::time::Duration;

use cosmwasm_std_old::Uint128;
use localic_std::modules::{bank::get_balance, cosmwasm::contract_query};
use localic_utils::{utils::test_context::TestContext, NEUTRON_CHAIN_NAME};
use log::info;
use valence_account_utils::ica::IcaState;

use super::relayer::restart_relayer;

#[allow(clippy::too_many_arguments)]
pub fn send_successful_ibc_transfer(
    test_ctx: &mut TestContext,
    origin_chain: &str,
    dest_chain: &str,
    amount: u128,
    origin_denom: &str,
    dest_denom: &str,
    recipient: &str,
    max_tries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tries = 0;
    loop {
        if tries >= max_tries {
            panic!("Failed to send IBC transfer after {max_tries} tries");
        }
        tries += 1;
        test_ctx
            .build_tx_transfer()
            .with_chain_name(origin_chain)
            .with_amount(amount)
            .with_recipient(recipient)
            .with_denom(origin_denom)
            .send()?;

        info!(
            "Waiting to receive {} IBC transfer on {}...",
            origin_denom, dest_chain
        );
        std::thread::sleep(std::time::Duration::from_secs(5));
        let balance = get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(dest_chain),
            recipient,
        );
        if balance
            .iter()
            .any(|c| c.denom == dest_denom && c.amount >= Uint128::new(amount))
        {
            info!("Received {} IBC transfer!", origin_denom);
            break;
        }
    }

    Ok(())
}

pub fn poll_for_ica_state<F>(test_ctx: &mut TestContext, addr: &str, expected: F)
where
    F: Fn(&IcaState) -> bool,
{
    let mut attempts = 0;
    loop {
        attempts += 1;
        let ica_state: IcaState = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                addr,
                &serde_json::to_string(&valence_account_utils::ica::QueryMsg::IcaState {}).unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        if expected(&ica_state) {
            info!("Target ICA state reached!");
            break;
        } else {
            info!(
                "Waiting for the right ICA state, current state: {:?}",
                ica_state
            );
        }

        if attempts % 5 == 0 {
            restart_relayer(test_ctx);
        }

        if attempts > 60 {
            panic!("Maximum number of attempts reached. Cancelling execution.");
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}
