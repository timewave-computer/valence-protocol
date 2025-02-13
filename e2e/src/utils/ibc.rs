use cosmwasm_std_old::Uint128;
use localic_std::modules::bank::get_balance;
use localic_utils::utils::test_context::TestContext;
use log::info;

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
            panic!("Failed to send IBC transfer after {} tries", max_tries);
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
