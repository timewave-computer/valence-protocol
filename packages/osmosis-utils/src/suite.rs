use cosmwasm_std::{Coin, StdResult, Uint64};

use osmosis_test_tube::{
    osmosis_std::{
        try_proto_to_cosmwasm_coins,
        types::cosmos::bank::v1beta1::{MsgSend, QueryAllBalancesRequest},
    },
    Account, Bank, Module, OsmosisTestApp, SigningAccount, Wasm,
};

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
    fn get_provider_contract_name() -> String;
    fn get_withdrawer_contract_name() -> String;
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
                    Coin::new(self.initial_balance, self.fee_denom.as_str()),
                    Coin::new(self.initial_balance, TEST_DENOM),
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
    pub fn approve_library(&self, account_addr: String, library_addr: String) {
        let wasm = Wasm::new(&self.app);
        wasm.execute::<valence_account_utils::msg::ExecuteMsg>(
            &account_addr,
            &valence_account_utils::msg::ExecuteMsg::ApproveLibrary {
                library: library_addr,
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
                approved_libraries: vec![],
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

    pub fn store_provider_contract(&self) -> u64 {
        let filename = T::get_provider_contract_name();
        println!("filename: {}", filename);
        let wasm = Wasm::new(&self.app);
        let wasm_byte_code = std::fs::read(format!("{}/{}", CONTRACT_PATH, filename)).unwrap();

        let code_id = wasm
            .store_code(&wasm_byte_code, None, self.owner_acc())
            .unwrap()
            .data
            .code_id;

        code_id
    }

    pub fn store_withdrawer_contract(&self) -> u64 {
        let filename = T::get_withdrawer_contract_name();
        let wasm = Wasm::new(&self.app);
        let wasm_byte_code = std::fs::read(format!("{}/{}", CONTRACT_PATH, filename)).unwrap();

        let code_id = wasm
            .store_code(&wasm_byte_code, None, self.owner_acc())
            .unwrap()
            .data
            .code_id;

        code_id
    }

    pub fn fund_input_acc(&self, input_acc: String, with_input_bals: Vec<Coin>) {
        let bank = Bank::new(&self.app);

        for input_bal in with_input_bals {
            bank.send(
                MsgSend {
                    from_address: self.owner_acc().address(),
                    to_address: input_acc.clone(),
                    amount: vec![
                        osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                            denom: input_bal.denom.clone(),
                            amount: input_bal.amount.to_string(),
                        },
                    ],
                },
                self.owner_acc(),
            )
            .unwrap();
        }
    }

    pub fn store_account_contract(&self) -> u64 {
        let wasm = Wasm::new(&self.app);

        let wasm_byte_code =
            std::fs::read(format!("{}/{}", CONTRACT_PATH, "valence_base_account.wasm")).unwrap();

        wasm.store_code(&wasm_byte_code, None, self.owner_acc())
            .unwrap()
            .data
            .code_id
    }

    pub fn query_all_balances(&self, addr: &str) -> StdResult<Vec<Coin>> {
        let bank = Bank::new(&self.app);
        let resp = bank
            .query_all_balances(&QueryAllBalancesRequest {
                address: addr.to_string(),
                pagination: None,
                resolve_denom: false,
            })
            .unwrap();
        try_proto_to_cosmwasm_coins(resp.balances)
    }
}
