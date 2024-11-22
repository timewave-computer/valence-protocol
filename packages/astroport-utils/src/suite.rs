use std::{collections::HashMap, path::PathBuf};

use cosmwasm_std::{coin, coins, Uint128};
use neutron_test_tube::{
    neutron_std::types::{
        cosmos::base::v1beta1::Coin,
        osmosis::tokenfactory::v1beta1::{MsgCreateDenom, MsgMint},
    },
    Account, Module, NeutronTestApp, SigningAccount, TokenFactory, Wasm,
};

pub const FEE_DENOM: &str = "untrn";
const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const CONTRACTS_DIR: &str = "packages/astroport-utils/contracts";

pub struct AstroportTestAppBuilder {
    fee_denom: String,
    initial_balance: u128,
    num_accounts: u64,
}

impl Default for AstroportTestAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AstroportTestAppBuilder {
    pub fn new() -> Self {
        AstroportTestAppBuilder {
            fee_denom: FEE_DENOM.to_string(),
            initial_balance: 100_000_000_000_000_000,
            num_accounts: 2,
        }
    }

    pub fn build(self) -> Result<AstroportTestAppSetup, &'static str> {
        let app = NeutronTestApp::new();

        let accounts = app
            .init_accounts(
                &coins(self.initial_balance, &self.fee_denom),
                self.num_accounts,
            )
            .map_err(|_| "Failed to initialize accounts")?;

        let wasm = Wasm::new(&app);

        // Create the full path to the contracts directory
        let contracts_path = PathBuf::from(PROJECT_ROOT)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(CONTRACTS_DIR);

        let wasm_files = [
            "astroport_factory_native.wasm",
            "astroport_factory_cw20.wasm",
            "astroport_pair_native.wasm",
            "astroport_pair_cw20.wasm",
            "astroport_token.wasm",
        ];

        let mut code_ids = HashMap::new();

        for file in wasm_files {
            // Load WASM code
            let wasm_path = contracts_path.join(file);
            let wasm_bytes = std::fs::read(wasm_path).map_err(|_| "Failed to read WASM file")?;

            // Store WASM code
            let code_id = wasm
                .store_code(&wasm_bytes, None, &accounts[0])
                .unwrap()
                .data
                .code_id;

            // Save code ID with trimmed filename (remove .wasm extension)
            let trimmed_name = file.trim_end_matches(".wasm").to_string();
            code_ids.insert(trimmed_name, code_id);
        }

        // Let's create a tokenfactory denom to pool with NTRN
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

        // Mint some tokens to the owner account so that we can provide liquidity and later on send some tokens for tests
        token_factory
            .mint(
                MsgMint {
                    sender: accounts[0].address().clone(),
                    amount: Some(Coin {
                        denom: denom.clone(),
                        amount: 1_000_000_000_000_000u128.to_string(),
                    }),
                    mint_to_address: accounts[0].address().clone(),
                },
                &accounts[0],
            )
            .unwrap();

        // Instantiate the first factory and create a pool with NTRN and the new denom as assets
        let factory_native_address = wasm
            .instantiate(
                *code_ids.get("astroport_factory_native").unwrap(),
                &crate::astroport_native_lp_token::FactoryInstantiateMsg {
                    pair_configs: vec![crate::astroport_native_lp_token::PairConfig {
                        code_id: *code_ids.get("astroport_pair_native").unwrap(),
                        pair_type: crate::astroport_native_lp_token::PairType::Xyk {},
                        total_fee_bps: 0,
                        maker_fee_bps: 0,
                        is_disabled: false,
                        is_generator_disabled: true,
                        permissioned: false,
                    }],
                    token_code_id: *code_ids.get("astroport_token").unwrap(),
                    fee_address: None,
                    generator_address: None,
                    owner: accounts[0].address().clone(),
                    whitelist_code_id: 0, // Not using it
                    coin_registry_address: accounts[0].address().clone(), // Any address will do to pass address validation
                    tracker_config: None,
                },
                None,
                "factory_native".into(),
                &[],
                &accounts[0],
            )
            .unwrap()
            .data
            .address;

        let assets = vec![
            crate::astroport_native_lp_token::AssetInfo::NativeToken {
                denom: FEE_DENOM.to_string(),
            },
            crate::astroport_native_lp_token::AssetInfo::NativeToken {
                denom: denom.clone(),
            },
        ];
        // Create a pool with NTRN and the denom as assets
        wasm.execute(
            &factory_native_address,
            &crate::astroport_native_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: crate::astroport_native_lp_token::PairType::Xyk {},
                asset_infos: assets.clone(),
                init_params: None,
            },
            &[],
            &accounts[0],
        )
        .unwrap();

        // Get the pool address
        let pair_info = wasm
            .query::<crate::astroport_native_lp_token::FactoryQueries, crate::astroport_native_lp_token::PairInfo>(
                &factory_native_address,
                &crate::astroport_native_lp_token::FactoryQueries::Pair {
                    asset_infos: assets,
                },
            )
            .unwrap();

        let pool_native_addr = pair_info.contract_addr.to_string();
        let pool_native_liquidity_token = pair_info.liquidity_token;

        // Provide some initial liquidity
        wasm.execute(
            &pool_native_addr,
            &crate::astroport_native_lp_token::ExecuteMsg::ProvideLiquidity {
                assets: vec![
                    crate::astroport_native_lp_token::Asset {
                        info: crate::astroport_native_lp_token::AssetInfo::NativeToken {
                            denom: FEE_DENOM.to_string(),
                        },
                        amount: Uint128::new(1_000_000_000),
                    },
                    crate::astroport_native_lp_token::Asset {
                        info: crate::astroport_native_lp_token::AssetInfo::NativeToken {
                            denom: denom.to_string(),
                        },
                        amount: Uint128::new(1_000_000_000),
                    },
                ],
                slippage_tolerance: None,
                auto_stake: Some(false),
                receiver: None,
                min_lp_to_receive: None,
            },
            &[
                coin(1_000_000_000u128, denom.clone()),
                coin(1_000_000_000u128, FEE_DENOM.to_string()),
            ],
            &accounts[0],
        )
        .unwrap();

        // Create now the pool using the old factory
        let factory_cw20_address = wasm
            .instantiate(
                *code_ids.get("astroport_factory_cw20").unwrap(),
                &crate::astroport_cw20_lp_token::FactoryInstantiateMsg {
                    pair_configs: vec![crate::astroport_cw20_lp_token::PairConfig {
                        code_id: *code_ids.get("astroport_pair_cw20").unwrap(),
                        pair_type: crate::astroport_cw20_lp_token::PairType::Xyk {},
                        total_fee_bps: 0,
                        maker_fee_bps: 0,
                        is_disabled: false,
                        is_generator_disabled: true,
                    }],
                    token_code_id: *code_ids.get("astroport_token").unwrap(),
                    fee_address: None,
                    generator_address: None,
                    owner: accounts[0].address().clone(),
                    whitelist_code_id: 0, // Not using it
                    coin_registry_address: accounts[0].address().clone(), // Any address will do to pass address validation
                },
                None,
                "factory_cw20".into(),
                &[],
                &accounts[0],
            )
            .unwrap()
            .data
            .address;

        let assets = vec![
            crate::astroport_cw20_lp_token::AssetInfo::NativeToken {
                denom: FEE_DENOM.to_string(),
            },
            crate::astroport_cw20_lp_token::AssetInfo::NativeToken {
                denom: denom.clone(),
            },
        ];
        // Create a pool with NTRN and the denom as assets
        wasm.execute(
            &factory_cw20_address,
            &crate::astroport_cw20_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: crate::astroport_cw20_lp_token::PairType::Xyk {},
                asset_infos: assets.clone(),
                init_params: None,
            },
            &[],
            &accounts[0],
        )
        .unwrap();

        // Get the pool address
        let pair_info = wasm
            .query::<crate::astroport_cw20_lp_token::FactoryQueries, crate::astroport_cw20_lp_token::PairInfo>(
                &factory_cw20_address,
                &crate::astroport_cw20_lp_token::FactoryQueries::Pair {
                    asset_infos: assets,
                },
            )
            .unwrap();

        let pool_cw20_addr = pair_info.contract_addr.to_string();
        let pool_cw20_liquidity_token = pair_info.liquidity_token.to_string();

        // Provide some initial liquidity
        wasm.execute(
            &pool_cw20_addr,
            &crate::astroport_cw20_lp_token::ExecuteMsg::ProvideLiquidity {
                assets: vec![
                    crate::astroport_cw20_lp_token::Asset {
                        info: crate::astroport_cw20_lp_token::AssetInfo::NativeToken {
                            denom: FEE_DENOM.to_string(),
                        },
                        amount: Uint128::new(1_000_000_000),
                    },
                    crate::astroport_cw20_lp_token::Asset {
                        info: crate::astroport_cw20_lp_token::AssetInfo::NativeToken {
                            denom: denom.clone(),
                        },
                        amount: Uint128::new(1_000_000_000),
                    },
                ],
                slippage_tolerance: None,
                auto_stake: Some(false),
                receiver: None,
            },
            &[
                coin(1_000_000_000u128, denom.clone()),
                coin(1_000_000_000u128, FEE_DENOM.to_string()),
            ],
            &accounts[0],
        )
        .unwrap();

        Ok(AstroportTestAppSetup {
            app,
            accounts,
            pool_native_addr,
            pool_native_liquidity_token,
            pool_cw20_addr,
            pool_cw20_liquidity_token,
            pool_asset1: FEE_DENOM.to_string(),
            pool_asset2: denom,
        })
    }
}

pub struct AstroportTestAppSetup {
    pub app: NeutronTestApp,
    pub accounts: Vec<SigningAccount>,
    pub pool_native_addr: String,
    pub pool_native_liquidity_token: String,
    pub pool_cw20_addr: String,
    pub pool_cw20_liquidity_token: String,
    pub pool_asset1: String,
    pub pool_asset2: String,
}

impl AstroportTestAppSetup {
    pub fn owner_acc(&self) -> &SigningAccount {
        &self.accounts[0]
    }

    pub fn processor_acc(&self) -> &SigningAccount {
        &self.accounts[1]
    }
}
