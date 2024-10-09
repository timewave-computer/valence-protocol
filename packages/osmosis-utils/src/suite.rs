use osmosis_test_tube::{Gamm, Module, OsmosisTestApp, SigningAccount};

pub const OSMO_DENOM: &str = "uosmo";

pub struct OsmosisTestAppSetup {
    pub app: OsmosisTestApp,
    pub accounts: Vec<SigningAccount>,
    pub pool_id: String,
    pub pool_liquidity_token: String,
    pub pool_asset1: String,
    pub pool_asset2: String,
}

impl OsmosisTestAppSetup {
    pub fn owner_acc(&self) -> &SigningAccount {
        &self.accounts[0]
    }

    pub fn processor_acc(&self) -> &SigningAccount {
        &self.accounts[1]
    }
}

pub struct OsmosisTestAppBuilder {
    fee_denom: String,
    initial_balance: u128,
    num_accounts: u64,
}

impl Default for OsmosisTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OsmosisTestAppBuilder {
    pub fn new() -> Self {
        Self {
            fee_denom: OSMO_DENOM.to_string(),
            initial_balance: 100_000_000_000_000_000,
            num_accounts: 2,
        }
    }

    pub fn build(self) -> Result<OsmosisTestAppSetup, &'static str> {
        let app = OsmosisTestApp::new();
        let gamm = Gamm::new(&app);

        let accounts = app
            .init_accounts(
                &[
                    cosmwasm_std_polytone::Coin::new(self.initial_balance, self.fee_denom.as_str()),
                    cosmwasm_std_polytone::Coin::new(self.initial_balance, "utest"),
                ],
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        // create balancer pool with basic configuration
        let pool_liquidity = vec![
            cosmwasm_std_polytone::Coin::new(100_000u128, self.fee_denom),
            cosmwasm_std_polytone::Coin::new(100_000u128, "utest"),
        ];
        let pool_id = gamm
            .create_basic_pool(&pool_liquidity, &accounts[0])
            .unwrap()
            .data
            .pool_id;

        let pool = gamm.query_pool(pool_id).unwrap();

        let pool_liquidity_token = pool.total_shares.unwrap().denom;

        Ok(OsmosisTestAppSetup {
            app,
            accounts,
            pool_id: pool_id.to_string(),
            pool_asset1: OSMO_DENOM.to_string(),
            pool_asset2: "utest".to_string(),
            pool_liquidity_token,
        })
    }
}
