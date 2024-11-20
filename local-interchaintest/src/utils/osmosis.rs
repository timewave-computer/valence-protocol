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

pub mod concentrated_liquidity {
    use std::error::Error;

    use localic_std::modules::bank;
    use localic_utils::{
        utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
        OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_NAME,
    };
    use log::info;
    use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::UserPositionsResponse;

    pub fn query_cl_position(
        test_ctx: &mut TestContext,
        addr: &str,
    ) -> Result<UserPositionsResponse, Box<dyn Error>> {
        info!("querying {addr} cl positions...");

        let cmd = format!("concentratedliquidity user-positions {addr} --output=json");

        let rb = test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME);

        let resp = rb.q(&cmd, false);

        let user_positions: UserPositionsResponse = serde_json::from_value(resp).unwrap();

        info!("{addr} CL positions: {:?}", user_positions.positions);

        Ok(user_positions)
    }

    pub fn setup_cl_pool(
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

        let osmo_rb = test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME);

        let tick_spacing = 1000;
        let spread_factor = 0.0005;

        let cmd = format!(
            "tx concentratedliquidity create-pool {denom_2} {denom_1} {tick_spacing} {spread_factor} --fees=5000uosmo --from={} --gas auto --gas-adjustment 1.3 --output=json",
            DEFAULT_KEY
        );
        info!("CL creation cmd: {cmd}");

        let cl_creation_response_events = osmo_rb.tx(&cmd, true)?["events"].clone();

        let pool_creation_response_event = cl_creation_response_events
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| match e["attributes"].clone() {
                serde_json::Value::Array(vec) => Some(vec),
                _ => None,
            })
            .flatten()
            .find(|e| e["key"] == "pool_id")
            .unwrap();

        let pool_id_str = match pool_creation_response_event["value"].clone() {
            serde_json::Value::String(n) => n.to_string(),
            _ => panic!("pool_id not found in cl creation response"),
        };

        let pool_id: u64 = pool_id_str.parse().unwrap();

        info!("CL pool id: {:?}", pool_id);
        // Usage:
        //   osmosisd tx concentratedliquidity create-position [pool-id] [lower-tick] [upper-tick] [tokens-provided] [token-min-amount0] [token-min-amount1] [flags]

        // Examples:
        // osmosisd tx concentratedliquidity create-position 1 "[-69082]" 69082 10000uosmo,10000uion 0 0 --from val --chain-id osmosis-1 -b block --keyring-backend test --fees 1000uosmo

        let lp_cmd = format!(
            "tx concentratedliquidity create-position {pool_id} 1000 2000 10000000uosmo,10000000{denom_2} 0 0 --from={} --fees=5000uosmo --gas auto --gas-adjustment 1.3 --output=json",
            DEFAULT_KEY
        );

        let lp_response = test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME)
            .tx(&lp_cmd, false)?;

        info!("initial LP response: {:?}", lp_response);

        Ok(pool_id)
    }
}
