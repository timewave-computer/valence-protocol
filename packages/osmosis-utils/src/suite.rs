use cosmwasm_std::Uint64;
use osmosis_test_tube::{Account, Gamm, Module, OsmosisTestApp, SigningAccount, Wasm};

pub const OSMO_DENOM: &str = "uosmo";

pub struct OsmosisTestAppSetup {
    pub app: OsmosisTestApp,
    pub accounts: Vec<SigningAccount>,
    pub pool_id: Uint64,
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
            pool_id: Uint64::new(pool_id),
            pool_asset1: OSMO_DENOM.to_string(),
            pool_asset2: "utest".to_string(),
            pool_liquidity_token,
        })
    }
}

pub fn approve_service(setup: &OsmosisTestAppSetup, account_addr: String, service_addr: String) {
    let wasm = Wasm::new(&setup.app);
    wasm.execute::<valence_account_utils::msg::ExecuteMsg>(
        &account_addr,
        &valence_account_utils::msg::ExecuteMsg::ApproveService {
            service: service_addr,
        },
        &[],
        setup.owner_acc(),
    )
    .unwrap();
}

pub fn instantiate_input_account(code_id: u64, setup: &OsmosisTestAppSetup) -> String {
    let wasm = Wasm::new(&setup.app);
    wasm.instantiate(
        code_id,
        &valence_account_utils::msg::InstantiateMsg {
            admin: setup.owner_acc().address(),
            approved_services: vec![],
        },
        None,
        Some("base_account"),
        &[],
        setup.owner_acc(),
    )
    .unwrap()
    .data
    .address
}
