use cosmwasm_std::Uint64;
use osmosis_test_tube::{
    osmosis_std::types::{
        cosmos::bank::v1beta1::MsgSend,
        osmosis::tokenfactory::v1beta1::{MsgCreateDenom, MsgMint},
    },
    Account, Bank, Gamm, Module, OsmosisTestApp, SigningAccount, TokenFactory, Wasm,
};
use valence_service_utils::msg::InstantiateMsg;

use crate::{msg::LiquidityProviderConfig, valence_service_integration::ServiceConfig};

const CONTRACT_PATH: &str = "../../../artifacts";
pub const FEE_DENOM: &str = "uosmo";
const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

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
            fee_denom: "uosmo".to_string(),
            initial_balance: 100_000_000_000_000_000,
            num_accounts: 2,
        }
    }

    pub fn build(self) -> Result<OsmosisTestAppSetup, &'static str> {
        let app = OsmosisTestApp::new();

        let accounts = app
            .init_accounts(
                &[cosmwasm_std_polytone::Coin {
                    denom: self.fee_denom,
                    amount: self.initial_balance.into(),
                }],
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        let wasm = Wasm::new(&app);

        let token_factory = TokenFactory::new(&app);
        let denom = token_factory
            .create_denom(
                MsgCreateDenom {
                    sender: accounts[0].address().clone(),
                    subdenom: "test".to_string(),
                },
                &accounts[0],
            )
            .unwrap()
            .data
            .new_token_denom;

        token_factory
            .mint(
                MsgMint {
                    sender: accounts[0].address().clone(),
                    amount: Some(
                        osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                            denom: denom.clone(),
                            amount: 1_000_000_000_000_000u128.to_string(),
                        },
                    ),
                    mint_to_address: accounts[0].address().clone(),
                },
                &accounts[0],
            )
            .unwrap();

        // create Gamm Module Wrapper
        let gamm = Gamm::new(&app);

        // create balancer pool with basic configuration
        let pool_liquidity = vec![
            cosmwasm_std_polytone::Coin::new(1_000u128, denom.clone()),
            cosmwasm_std_polytone::Coin::new(1_000u128, "uosmo"),
        ];
        let pool_id = gamm
            .create_basic_pool(&pool_liquidity, &accounts[0])
            .unwrap()
            .data
            .pool_id;

        // query pool and assert if the pool is created successfully
        let pool = gamm.query_pool(pool_id).unwrap();
        assert_eq!(
            pool_liquidity
                .into_iter()
                .map(|c| c.into())
                .collect::<Vec<osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin>>(
                ),
            pool.pool_assets
                .into_iter()
                .map(|a| a.token.unwrap())
                .collect::<Vec<osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin>>(
                ),
        );

        // Mint some tokens to the owner account so that we can provide liquidity and later on send some tokens for tests
        token_factory
            .mint(
                MsgMint {
                    sender: accounts[0].address().clone(),
                    amount: Some(
                        osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                            denom: denom.clone(),
                            amount: 1_000_000_000_000_000u128.to_string(),
                        },
                    ),
                    mint_to_address: accounts[0].address().clone(),
                },
                &accounts[0],
            )
            .unwrap();

        Ok(OsmosisTestAppSetup {
            app,
            accounts,
            pool_id: pool_id.to_string(),
            pool_asset1: FEE_DENOM.to_string(),
            pool_asset2: denom,
            pool_liquidity_token: pool.total_shares.unwrap().denom,
        })
    }
}

pub(crate) struct LPerTestSuite {
    pub inner: OsmosisTestAppSetup,
    pub lper_addr: String,
    pub input_acc: String,
    pub output_acc: String,
}

impl Default for LPerTestSuite {
    fn default() -> Self {
        Self::new(true)
    }
}

impl LPerTestSuite {
    pub fn new(native_lp_token: bool) -> Self {
        let inner = OsmosisTestAppBuilder::new().build().unwrap();

        // Create two base accounts
        let wasm = Wasm::new(&inner.app);

        let wasm_byte_code =
            std::fs::read(format!("{}/{}", CONTRACT_PATH, "valence_base_account.wasm")).unwrap();
        let code_id = wasm
            .store_code(&wasm_byte_code, None, inner.owner_acc())
            .unwrap()
            .data
            .code_id;

        let input_acc = instantiate_input_account(code_id, &inner);
        let output_acc = instantiate_input_account(code_id, &inner);
        let lper_addr = instantiate_lper_contract(
            &inner,
            native_lp_token,
            input_acc.clone(),
            output_acc.clone(),
        );

        // Approve the service for the input account
        approve_service(&inner, input_acc.clone(), lper_addr.clone());

        // Give some tokens to the input account so that it can provide liquidity
        let bank = Bank::new(&inner.app);
        bank.send(
            MsgSend {
                from_address: inner.owner_acc().address(),
                to_address: input_acc.clone(),
                amount: vec![
                    osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                        denom: inner.pool_asset2.clone(),
                        amount: 1_000_000u128.to_string(),
                    },
                    osmosis_test_tube::osmosis_std::types::cosmos::base::v1beta1::Coin {
                        denom: inner.pool_asset1.clone(),
                        amount: 1_000_000u128.to_string(),
                    },
                ],
            },
            inner.owner_acc(),
        )
        .unwrap();

        LPerTestSuite {
            inner,
            lper_addr,
            input_acc,
            output_acc,
        }
    }
}

fn approve_service(setup: &OsmosisTestAppSetup, account_addr: String, service_addr: String) {
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

fn instantiate_input_account(code_id: u64, setup: &OsmosisTestAppSetup) -> String {
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

fn instantiate_lper_contract(
    setup: &OsmosisTestAppSetup,
    native_lp_token: bool,
    input_acc: String,
    output_acc: String,
) -> String {
    let wasm = Wasm::new(&setup.app);
    let wasm_byte_code =
        std::fs::read(format!("{}/{}", CONTRACT_PATH, "valence_osmosis_lper.wasm")).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, setup.owner_acc())
        .unwrap()
        .data
        .code_id;

    let pool_addr = "todo".to_string();
    let pool_id = Uint64::one();

    wasm.instantiate(
        code_id,
        &InstantiateMsg {
            owner: setup.owner_acc().address(),
            processor: setup.processor_acc().address(),
            config: ServiceConfig::new(
                input_acc.as_str(),
                output_acc.as_str(),
                pool_addr,
                LiquidityProviderConfig {
                    pool_id: pool_id.into(),
                },
            ),
        },
        None,
        Some("lper"),
        &[],
        setup.owner_acc(),
    )
    .unwrap()
    .data
    .address
}
