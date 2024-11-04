use cosmwasm_std::{Coin, StdResult, Uint64};
use osmosis_test_tube::{Gamm, Module, OsmosisTestApp, SigningAccount};

use crate::suite::{OsmosisTestPoolConfig, OSMO_DENOM, TEST_DENOM};

pub struct BalancerPool {
    pub pool_id: Uint64,
    pub pool_liquidity_token: String,
    pub pool_asset1: String,
    pub pool_asset2: String,
}

impl BalancerPool {}

impl OsmosisTestPoolConfig for BalancerPool {
    fn pool_id(&self) -> Uint64 {
        self.pool_id
    }

    fn pool_asset_1(&self) -> String {
        self.pool_asset1.clone()
    }

    fn pool_asset_2(&self) -> String {
        self.pool_asset2.clone()
    }

    fn setup_pool(app: &OsmosisTestApp, creator: &SigningAccount) -> StdResult<Self> {
        let gamm = Gamm::new(app);

        // create balancer pool with basic configuration
        let pool_liquidity = vec![
            Coin::new(100_000u128, OSMO_DENOM),
            Coin::new(100_000u128, TEST_DENOM),
        ];
        let pool_id = gamm
            .create_basic_pool(&pool_liquidity, creator)
            .unwrap()
            .data
            .pool_id;

        let pool = gamm.query_pool(pool_id).unwrap();

        let pool_liquidity_token = pool.total_shares.unwrap().denom;

        let balancer_pool = BalancerPool {
            pool_id: pool_id.into(),
            pool_liquidity_token,
            pool_asset1: OSMO_DENOM.to_string(),
            pool_asset2: TEST_DENOM.to_string(),
        };

        Ok(balancer_pool)
    }

    fn get_provider_contract_name() -> String {
        "valence_osmosis_gamm_lper.wasm".to_string()
    }

    fn get_withdrawer_contract_name() -> String {
        "valence_osmosis_gamm_withdrawer.wasm".to_string()
    }
}
