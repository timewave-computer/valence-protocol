use cosmwasm_std::{StdResult, Uint64};

use osmosis_test_tube::{Account, Gamm, Module, OsmosisTestApp, SigningAccount, Wasm};

pub const OSMO_DENOM: &str = "uosmo";
pub const TEST_DENOM: &str = "utest";

pub struct OsmosisTestAppSetup {
    pub app: OsmosisTestApp,
    pub accounts: Vec<SigningAccount>,
    pub balancer_pool_cfg: BalancerPool,
}

pub struct BalancerPool {
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

        let accounts = app
            .init_accounts(
                &[
                    cosmwasm_std_polytone::Coin::new(self.initial_balance, self.fee_denom.as_str()),
                    cosmwasm_std_polytone::Coin::new(self.initial_balance, TEST_DENOM),
                ],
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        let balancer_pool = setup_balancer_pool(&app, &accounts[0]).unwrap();

        Ok(OsmosisTestAppSetup {
            app,
            accounts,
            balancer_pool_cfg: balancer_pool,
        })
    }
}

fn setup_balancer_pool(app: &OsmosisTestApp, creator: &SigningAccount) -> StdResult<BalancerPool> {
    let gamm = Gamm::new(app);

    // create balancer pool with basic configuration
    let pool_liquidity = vec![
        cosmwasm_std_polytone::Coin::new(100_000u128, OSMO_DENOM),
        cosmwasm_std_polytone::Coin::new(100_000u128, TEST_DENOM),
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
