pub mod gamm {
    use std::error::Error;

    use localic_std::modules::bank;
    use localic_utils::{
        utils::test_context::TestContext, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
        OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_NAME,
    };
    use log::info;

    pub fn setup_gamm_pool(
        test_ctx: &mut TestContext,
        denom_1: &str,
        denom_2: &str,
    ) -> Result<u64, Box<dyn Error>> {
        info!("transferring 1000 neutron tokens to osmo admin addr for pool creation...");
        test_ctx
            .build_tx_transfer()
            .with_chain_name(NEUTRON_CHAIN_NAME)
            .with_amount(1_000_000_000u128)
            .with_recipient(OSMOSIS_CHAIN_ADMIN_ADDR)
            .with_denom(NEUTRON_CHAIN_DENOM)
            .send()?;
        std::thread::sleep(std::time::Duration::from_secs(3));

        let token_balances = bank::get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(OSMOSIS_CHAIN_NAME),
            OSMOSIS_CHAIN_ADMIN_ADDR,
        );
        info!("osmosis chain admin addr balances: {:?}", token_balances);

        test_ctx
            .build_tx_create_osmo_pool()
            .with_weight(denom_1, 1)
            .with_weight(denom_2, 1)
            .with_initial_deposit(denom_1, 100_000_000)
            .with_initial_deposit(denom_2, 100_000_000)
            .send()?;

        // Get its id
        let pool_id = test_ctx
            .get_osmo_pool()
            .denoms(denom_1.into(), denom_2.to_string())
            .get_u64();

        info!("Gamm pool id: {:?}", pool_id);

        Ok(pool_id)
    }
}
