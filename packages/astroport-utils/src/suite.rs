use std::{collections::HashMap, path::PathBuf};

use cosmwasm_std::coins;
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

        // Mint some to the input account so that it can eventually provide liquidity to the pools
        token_factory
            .mint(
                MsgMint {
                    sender: accounts[0].address().clone(),
                    amount: Some(Coin {
                        denom: denom.clone(),
                        amount: 1_000_000_000_000u128.to_string(),
                    }),
                    mint_to_address: accounts[1].address().clone(),
                },
                &accounts[0],
            )
            .unwrap();

        // Instantiate the first factory and create a pool with NTRN and the new denom as assets
        let factory_native_address = wasm
            .instantiate(
                *code_ids.get("astroport_factory_native").unwrap(),
                &astroport::factory::InstantiateMsg {
                    pair_configs: vec![astroport::factory::PairConfig {
                        code_id: *code_ids.get("astroport_pair_native").unwrap(),
                        pair_type: astroport::factory::PairType::Xyk {},
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
            astroport::asset::AssetInfo::NativeToken {
                denom: FEE_DENOM.to_string(),
            },
            astroport::asset::AssetInfo::NativeToken {
                denom: denom.clone(),
            },
        ];
        // Create a pool with NTRN and the denom as assets
        wasm.execute(
            &factory_native_address,
            &astroport::factory::ExecuteMsg::CreatePair {
                pair_type: astroport::factory::PairType::Xyk {},
                asset_infos: assets.clone(),
                init_params: None,
            },
            &[],
            &accounts[0],
        )
        .unwrap();

        // Get the pool address
        let pair_info = wasm
            .query::<astroport::factory::QueryMsg, astroport::asset::PairInfo>(
                &factory_native_address,
                &astroport::factory::QueryMsg::Pair {
                    asset_infos: assets,
                },
            )
            .unwrap();

        let pool_native_addr = pair_info.contract_addr.to_string();
        let pool_native_liquidity_token = pair_info.liquidity_token;

        // Create now the pool using the old factory
        let factory_cw20_address = wasm
            .instantiate(
                *code_ids.get("astroport_factory_cw20").unwrap(),
                &astroport_cw20_lp_token::factory::InstantiateMsg {
                    pair_configs: vec![astroport_cw20_lp_token::factory::PairConfig {
                        code_id: *code_ids.get("astroport_pair_cw20").unwrap(),
                        pair_type: astroport_cw20_lp_token::factory::PairType::Xyk {},
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
            astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                denom: FEE_DENOM.to_string(),
            },
            astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                denom: denom.clone(),
            },
        ];
        // Create a pool with NTRN and the denom as assets
        wasm.execute(
            &factory_cw20_address,
            &astroport_cw20_lp_token::factory::ExecuteMsg::CreatePair {
                pair_type: astroport_cw20_lp_token::factory::PairType::Xyk {},
                asset_infos: assets.clone(),
                init_params: None,
            },
            &[],
            &accounts[0],
        )
        .unwrap();

        // Get the pool address
        let pair_info = wasm
            .query::<astroport_cw20_lp_token::factory::QueryMsg, astroport_cw20_lp_token::asset::PairInfo>(
                &factory_native_address,
                &astroport_cw20_lp_token::factory::QueryMsg::Pair {
                    asset_infos: assets,
                },
            )
            .unwrap();

        let pool_cw20_addr = pair_info.contract_addr.to_string();
        let pool_cw20_liquidity_token = pair_info.liquidity_token.to_string();

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
