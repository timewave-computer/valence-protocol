use std::{env, error::Error, str::FromStr};

use cosmwasm_std::{coin, to_json_binary, Decimal};
use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate, contract_query};
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_NAME,
};

use log::info;
use serde_json::Value;
use valence_astroport_utils::astroport_native_lp_token::{
    Asset, AssetInfo, ConcentratedLiquidityExecuteMsg, ConcentratedPoolParams,
    FactoryInstantiateMsg, FactoryQueryMsg, NativeCoinRegistryExecuteMsg,
    NativeCoinRegistryInstantiateMsg, PairConfig, PairType,
};

use valence_e2e::utils::{ASTROPORT_PATH, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON};

use crate::ASTROPORT_CONCENTRATED_PAIR_TYPE;

fn deploy_astroport_contracts(
    test_ctx: &mut TestContext,
) -> Result<(u64, u64, u64, u64), Box<dyn Error>> {
    info!("Uploading astroport contracts...");
    let current_dir = env::current_dir()?;
    let astroport_contracts_path = format!("{}/{}", current_dir.display(), ASTROPORT_PATH);

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(&astroport_contracts_path, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)?;

    // Set up the astroport factory and the pool
    let astroport_factory_code_id = test_ctx
        .get_contract()
        .contract("astroport_factory")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_pair_concentrated_code_id = test_ctx
        .get_contract()
        .contract("astroport_pair_concentrated")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_token_code_id = test_ctx
        .get_contract()
        .contract("astroport_token")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_coin_registry_code_id = test_ctx
        .get_contract()
        .contract("astroport_native_coin_registry")
        .get_cw()
        .code_id
        .unwrap();

    Ok((
        astroport_factory_code_id,
        astroport_pair_concentrated_code_id,
        astroport_token_code_id,
        astroport_coin_registry_code_id,
    ))
}

pub fn setup_astroport_cl_pool(
    test_ctx: &mut TestContext,
    uusdc_on_neutron: String,
) -> Result<(String, String), Box<dyn Error>> {
    let (
        astroport_factory_code_id,
        astroport_pair_concentrated_code_id,
        astroport_token_code_id,
        astroport_coin_registry_code_id,
    ) = deploy_astroport_contracts(test_ctx)?;

    info!("Instantiating astroport native coin registry...");
    let coin_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_coin_registry_code_id,
        &serde_json::to_string(&NativeCoinRegistryInstantiateMsg {
            owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        })
        .unwrap(),
        "astro_native_coin_registry",
        None,
        "",
    )
    .unwrap();

    info!(
        "Astroport native coin registry address: {}",
        coin_registry_contract.address.clone()
    );

    info!("whitelisting coin registry native coins...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &coin_registry_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(&NativeCoinRegistryExecuteMsg::Add {
            native_coins: vec![
                (NEUTRON_CHAIN_DENOM.to_string(), 6),
                (uusdc_on_neutron.to_string(), 6),
            ],
        })
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Instantiating astroport factory...");
    let astroport_factory_instantiate_msg = FactoryInstantiateMsg {
        pair_configs: vec![PairConfig {
            code_id: astroport_pair_concentrated_code_id,
            pair_type: PairType::Custom(ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string()),
            total_fee_bps: 0u16,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: false,
            permissioned: false,
        }],
        fee_address: None,
        generator_address: None,
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        whitelist_code_id: 234, // This is not needed anymore but still part of API
        coin_registry_address: coin_registry_contract.address.to_string(),
        tracker_config: None,
        token_code_id: astroport_token_code_id,
    };

    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_factory_code_id,
        &serde_json::to_string(&astroport_factory_instantiate_msg).unwrap(),
        "astroport_factory",
        None,
        "",
    )
    .unwrap();

    info!(
        "Astroport factory address: {}",
        factory_contract.address.clone()
    );

    info!("Create the pool...");
    let pool_assets = vec![
        AssetInfo::NativeToken {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: uusdc_on_neutron.clone(),
        },
    ];

    let default_params = ConcentratedPoolParams {
        amp: Decimal::from_ratio(40u128, 1u128),
        gamma: Decimal::from_ratio(145u128, 1000000u128),
        mid_fee: Decimal::from_str("0.0026").unwrap(),
        out_fee: Decimal::from_str("0.0045").unwrap(),
        fee_gamma: Decimal::from_ratio(23u128, 100000u128),
        repeg_profit_threshold: Decimal::from_ratio(2u128, 1000000u128),
        min_price_scale_delta: Decimal::from_ratio(146u128, 1000000u128),
        price_scale: Decimal::one(),
        ma_half_time: 600,
        track_asset_balances: None,
        fee_share: None,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_astroport_utils::astroport_native_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: PairType::Custom(ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string()),
                asset_infos: pool_assets.clone(),
                init_params: Some(to_json_binary(&default_params).unwrap()),
            },
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let query_pool_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &factory_contract.address.clone(),
            &serde_json::to_string(&FactoryQueryMsg::Pair {
                asset_infos: pool_assets.clone(),
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let pool_addr = query_pool_response["contract_addr"].as_str().unwrap();
    let lp_token = query_pool_response["liquidity_token"].as_str().unwrap();

    info!("Pool created successfully! Pool address: {pool_addr}, LP token: {lp_token}");
    let asset_a = coin(899_000_000, NEUTRON_CHAIN_DENOM);
    let asset_b = coin(899_000_000, uusdc_on_neutron.clone());
    let assets = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: asset_a.denom.to_string(),
            },
            amount: asset_a.amount,
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: asset_b.denom.to_string(),
            },
            amount: asset_b.amount,
        },
    ];

    let initial_lp_msg = ConcentratedLiquidityExecuteMsg::ProvideLiquidity {
        assets,
        slippage_tolerance: None,
        auto_stake: None,
        receiver: None,
        min_lp_to_receive: None,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        pool_addr,
        DEFAULT_KEY,
        &serde_json::to_string(&initial_lp_msg).unwrap(),
        &format!(
            "--amount {}{},{}{} --gas 1000000",
            asset_a.amount.u128(),
            asset_a.denom,
            asset_b.amount.u128(),
            asset_b.denom
        ),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok((pool_addr.to_string(), lp_token.to_string()))
}

pub mod ica {
    use std::{error::Error, time::Duration};

    use cosmwasm_std::Uint64;
    use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate, contract_query};
    use localic_utils::{
        utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR,
        NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    };
    use log::info;
    use valence_account_utils::ica::{IcaState, RemoteDomainInfo};
    use valence_e2e::utils::{
        ibc::poll_for_ica_state, manager::INTERCHAIN_ACCOUNT_NAME, GAS_FLAGS, NOBLE_CHAIN_NAME,
    };

    pub fn instantiate_interchain_account_contract(
        test_ctx: &TestContext,
    ) -> Result<String, Box<dyn Error>> {
        let ica_account_code = *test_ctx
            .get_chain(NEUTRON_CHAIN_NAME)
            .contract_codes
            .get(INTERCHAIN_ACCOUNT_NAME)
            .unwrap();

        info!("Instantiating the ICA contract...");
        let timeout_seconds = 90;
        let ica_instantiate_msg = valence_account_utils::ica::InstantiateMsg {
            admin: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
            approved_libraries: vec![],
            remote_domain_information: RemoteDomainInfo {
                connection_id: test_ctx
                    .get_connections()
                    .src(NEUTRON_CHAIN_NAME)
                    .dest(NOBLE_CHAIN_NAME)
                    .get(),
                ica_timeout_seconds: Uint64::new(timeout_seconds),
            },
        };

        let valence_ica = contract_instantiate(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            ica_account_code,
            &serde_json::to_string(&ica_instantiate_msg)?,
            "valence_ica",
            None,
            "",
        )?;
        info!(
            "ICA contract instantiated. Address: {}",
            valence_ica.address
        );

        Ok(valence_ica.address)
    }

    pub fn register_interchain_account(
        test_ctx: &mut TestContext,
        interchain_account_addr: &str,
    ) -> Result<String, Box<dyn Error>> {
        info!("Registering the ICA...");
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            interchain_account_addr,
            DEFAULT_KEY,
            &serde_json::to_string(&valence_account_utils::ica::ExecuteMsg::RegisterIca {})
                .unwrap(),
            &format!("{} --amount=100000000{}", GAS_FLAGS, NEUTRON_CHAIN_DENOM),
        )
        .unwrap();
        std::thread::sleep(Duration::from_secs(3));

        // We want to check that it's in state created
        poll_for_ica_state(test_ctx, interchain_account_addr, |state| {
            matches!(state, IcaState::Created(_))
        });

        // Get the remote address
        let ica_state: IcaState = serde_json::from_value(
            contract_query(
                test_ctx
                    .get_request_builder()
                    .get_request_builder(NEUTRON_CHAIN_NAME),
                interchain_account_addr,
                &serde_json::to_string(&valence_account_utils::ica::QueryMsg::IcaState {}).unwrap(),
            )["data"]
                .clone(),
        )
        .unwrap();

        let remote_address = match ica_state {
            IcaState::Created(ica_info) => ica_info.address,
            _ => {
                unreachable!("Expected IcaState::Created variant");
            }
        };
        info!("Remote address created: {}", remote_address);

        Ok(remote_address)
    }
}
