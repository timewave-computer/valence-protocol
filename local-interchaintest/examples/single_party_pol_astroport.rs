use std::{env, error::Error, time::SystemTime};

use cosmwasm_std_old::Coin;
use local_interchaintest::utils::{
    authorization::set_up_authorization_and_processor,
    base_account::{approve_service, create_base_accounts},
    ASTROPORT_PATH, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, NTRN_DENOM,
    VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate, contract_query},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
use rand::{distributions::Alphanumeric, Rng};
use serde_json::Value;
use valence_reverse_splitter_service::msg::ServiceConfig as ReverseSplitterServiceConfig;
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

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
        .contract("astroport_factory_native")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_pair_native_code_id = test_ctx
        .get_contract()
        .contract("astroport_pair_native")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_token_code_id = test_ctx
        .get_contract()
        .contract("astroport_token")
        .get_cw()
        .code_id
        .unwrap();

    info!("Instantiating astroport factory...");
    let astroport_factory_instantiate_msg =
        valence_astroport_utils::astroport_native_lp_token::FactoryInstantiateMsg {
            pair_configs: vec![
                valence_astroport_utils::astroport_native_lp_token::PairConfig {
                    code_id: astroport_pair_native_code_id,
                    pair_type: valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
                    total_fee_bps: 0,
                    maker_fee_bps: 0,
                    is_disabled: false,
                    is_generator_disabled: true,
                    permissioned: false,
                },
            ],
            token_code_id: astroport_token_code_id,
            fee_address: None,
            generator_address: None,
            owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
            whitelist_code_id: 0, // This is not needed anymore but still part of API
            coin_registry_address: NEUTRON_CHAIN_ADMIN_ADDR.to_string(), // Passing any address here is fine as long as it's a valid one
            tracker_config: None,
        };

    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_factory_code_id,
        &serde_json::to_string(&astroport_factory_instantiate_msg).unwrap(),
        "processor",
        None,
        "",
    )
    .unwrap();
    info!(
        "Astroport factory address: {}",
        factory_contract.address.clone()
    );

    // Let's create a token to pair it with NTRN
    let token_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token_subdenom)
        .get();

    info!("Create the pool...");
    let pool_assets = vec![
        valence_astroport_utils::astroport_native_lp_token::AssetInfo::NativeToken {
            denom: NTRN_DENOM.to_string(),
        },
        valence_astroport_utils::astroport_native_lp_token::AssetInfo::NativeToken {
            denom: token.clone(),
        },
    ];
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_astroport_utils::astroport_native_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: valence_astroport_utils::astroport_native_lp_token::PairType::Xyk {},
                asset_infos: pool_assets.clone(),
                init_params: None,
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
            &serde_json::to_string(
                &valence_astroport_utils::astroport_native_lp_token::FactoryQueryMsg::Pair {
                    asset_infos: pool_assets.clone(),
                },
            )
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let pool_addr = query_pool_response["contract_addr"].as_str().unwrap();
    let lp_token = query_pool_response["liquidity_token"].as_str().unwrap();

    info!(
        "Pool created successfully! Pool address: {}, LP token: {}",
        pool_addr, lp_token
    );

    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );
    // Upload and instantiate authorization and processor on Neutron
    let (authorization_contract_address, processor_contract_address) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    // Let's upload the base account contract to Neutron
    let current_dir = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&base_account_contract_path)?;

    // Get the code id
    let code_id_base_account = test_ctx
        .get_contract()
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    info!("Create all required base accounts...");
    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        code_id_base_account,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        10,
    );

    // All accounts that will be used in our workflow
    let tokenfactory_depositor = base_accounts[0].clone();
    let neutron_depositor = base_accounts[1].clone();
    let joint_deposit = base_accounts[2].clone();
    let lp_account = base_accounts[3].clone();
    let lp_position_account = base_accounts[4].clone();
    let available_lp_tokens_account = base_accounts[5].clone();
    let lp_withdrawal_account = base_accounts[6].clone();
    let withdrawal_account = base_accounts[7].clone();
    let tokenfactory_receiver = base_accounts[8].clone();
    let neutron_receiver = base_accounts[9].clone();

    info!("Upload all services that we need...");
    let splitter_contract_path = format!(
        "{}/artifacts/valence_splitter_service.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&splitter_contract_path)?;
    let code_id_splitter = test_ctx
        .get_contract()
        .contract("valence_splitter_service")
        .get_cw()
        .code_id
        .unwrap();

    let reverse_splitter_contract_path = format!(
        "{}/artifacts/valence_reverse_splitter_service.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&reverse_splitter_contract_path)?;
    let code_id_reverse_splitter = test_ctx
        .get_contract()
        .contract("valence_reverse_splitter_service")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_withdrawer_contract_path = format!(
        "{}/artifacts/valence_astroport_withdrawer.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&astroport_withdrawer_contract_path)?;
    let code_id_astroport_withdrawer = test_ctx
        .get_contract()
        .contract("valence_astroport_withdrawer")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_lper_contract_path = format!(
        "{}/artifacts/valence_astroport_lper.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&astroport_lper_contract_path)?;
    let code_id_astroport_lper = test_ctx
        .get_contract()
        .contract("valence_astroport_lper")
        .get_cw()
        .code_id
        .unwrap();

    info!("Fund the depositor accounts with the required tokens...");
    let lp_amount = 1_000_000_000u128;
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &tokenfactory_depositor,
        &[Coin {
            denom: token.clone(),
            amount: lp_amount.into(),
        }],
        &Coin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &neutron_depositor,
        &[Coin {
            denom: NTRN_DENOM.to_string(),
            amount: lp_amount.into(),
        }],
        &Coin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    info!("Depositor accounts funded successfully!");

    info!("Instantiate all services...");
    let reverse_splitter_instantiate_msg = valence_service_utils::msg::InstantiateMsg::<
        ReverseSplitterServiceConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: processor_contract_address.clone(),
        config: ReverseSplitterServiceConfig {
            output_addr: ServiceAccountType::Addr(joint_deposit.clone()),
            splits: vec![
                valence_reverse_splitter_service::msg::UncheckedSplitConfig {
                    denom: UncheckedDenom::Native(token.clone()),
                    account: ServiceAccountType::Addr(tokenfactory_depositor.clone()),
                    amount:
                        valence_reverse_splitter_service::msg::UncheckedSplitAmount::FixedAmount(
                            lp_amount.into(),
                        ),
                    factor: None,
                },
            ],
            base_denom: UncheckedDenom::Native(token.clone()),
        },
    };
    let reverse_splitter = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_splitter,
        &serde_json::to_string(&reverse_splitter_instantiate_msg).unwrap(),
        "reverse-splitter",
        None,
        "",
    )
    .unwrap();

    info!(
        "Reverse splitter instantiated! Address: {}",
        reverse_splitter.address
    );

    // Approve the service for both accounts
    approve_service(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &tokenfactory_depositor,
        reverse_splitter.address.clone(),
    );
    approve_service(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &neutron_depositor,
        reverse_splitter.address.clone(),
    );

    Ok(())
}
