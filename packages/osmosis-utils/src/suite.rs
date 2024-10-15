use cosmwasm_std::{StdResult, Uint64};

use osmosis_test_tube::{Account, Module, OsmosisTestApp, SigningAccount, Wasm};

pub const OSMO_DENOM: &str = "uosmo";
pub const TEST_DENOM: &str = "utest";
pub const CONTRACT_PATH: &str = "../../../artifacts";

pub struct OsmosisTestAppSetup<T: OsmosisTestPoolConfig> {
    pub app: OsmosisTestApp,
    pub accounts: Vec<SigningAccount>,
    pub pool_cfg: T,
}

pub trait OsmosisTestPoolConfig: Sized {
    fn pool_id(&self) -> Uint64;
    fn pool_asset_1(&self) -> String;
    fn pool_asset_2(&self) -> String;
    fn setup_pool(app: &OsmosisTestApp, creator: &SigningAccount) -> StdResult<Self>;
    fn get_contract_name() -> String;
}

impl<T: OsmosisTestPoolConfig> OsmosisTestAppSetup<T> {
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

    pub fn build<T: OsmosisTestPoolConfig>(self) -> Result<OsmosisTestAppSetup<T>, &'static str> {
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

        let pool_cfg = T::setup_pool(&app, &accounts[0]).map_err(|_| "failed to set up pool")?;

        Ok(OsmosisTestAppSetup {
            app,
            accounts,
            pool_cfg,
        })
    }
}

impl<T: OsmosisTestPoolConfig> OsmosisTestAppSetup<T> {
    pub fn approve_service(&self, account_addr: String, service_addr: String) {
        let wasm = Wasm::new(&self.app);
        wasm.execute::<valence_account_utils::msg::ExecuteMsg>(
            &account_addr,
            &valence_account_utils::msg::ExecuteMsg::ApproveService {
                service: service_addr,
            },
            &[],
            self.owner_acc(),
        )
        .unwrap();
    }

    pub fn instantiate_input_account(&self, code_id: u64) -> String {
        let wasm = Wasm::new(&self.app);
        wasm.instantiate(
            code_id,
            &valence_account_utils::msg::InstantiateMsg {
                admin: self.owner_acc().address(),
                approved_services: vec![],
            },
            None,
            Some("base_account"),
            &[],
            self.owner_acc(),
        )
        .unwrap()
        .data
        .address
    }

    pub fn store_contract(&self) -> u64 {
        let filename = T::get_contract_name();
        let wasm = Wasm::new(&self.app);
        let wasm_byte_code = std::fs::read(format!("{}/{}", CONTRACT_PATH, filename)).unwrap();

        let code_id = wasm
            .store_code(&wasm_byte_code, None, self.owner_acc())
            .unwrap()
            .data
            .code_id;

        code_id
    }
}
