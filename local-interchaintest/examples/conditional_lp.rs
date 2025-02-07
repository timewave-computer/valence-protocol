use cosmwasm_std::{to_json_binary, Binary, Coin, Uint64};
use cosmwasm_std::{Decimal, Uint128};
use cosmwasm_std_old::Coin as BankCoin;
use local_interchaintest::utils::base_account::create_base_accounts;
use local_interchaintest::utils::ibc::send_successful_ibc_transfer;
use local_interchaintest::utils::processor::tick_processor;
use local_interchaintest::utils::NTRN_DENOM;
use local_interchaintest::utils::{
    authorization::{set_up_authorization_and_processor, set_up_external_domain_with_polytone},
    base_account::create_storage_accounts,
    icq::{generate_icq_relayer_config, start_icq_relayer},
    osmosis::gamm::setup_gamm_pool,
    processor::confirm_remote_domain_processor_queue_length,
    GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_OSMOSIS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::{
    errors::LocalError,
    modules::{
        bank,
        cosmwasm::{contract_execute, contract_instantiate, contract_query},
    },
    types::TransactionResponse,
};
use log::info;
use serde_json::Value;
use std::str::FromStr;
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use valence_authorization_utils::domain::Domain;
use valence_authorization_utils::{
    authorization::Priority,
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_generic_ibc_transfer_library::msg::IbcTransferAmount;
use valence_library_utils::denoms::UncheckedDenom;
use valence_library_utils::LibraryAccountType;
use valence_middleware_asserter::msg::AssertionConfig;
use valence_middleware_utils::canonical_types::pools::xyk::XykPoolQuery;
use valence_middleware_utils::type_registry::types::{
    RegistryInstantiateMsg, RegistryQueryMsg, ValenceType,
};
use valence_neutron_ic_querier::msg::{FunctionMsgs, LibraryConfig, QueryDefinition};
use valence_processor_utils::processor::MessageBatch;

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_ID, OSMOSIS_CHAIN_NAME,
    OSMOSIS_CHAIN_PREFIX,
};

const TARGET_QUERY_LABEL: &str = "gamm_pool";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    let salt = hex::encode(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );
    info!("using salt: {salt}");

    let (authorization_contract_address, neutron_processor_address) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    info!("setting up external domain with polytone...");
    let processor_on_osmosis = set_up_external_domain_with_polytone(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        OSMOSIS_CHAIN_ID,
        OSMOSIS_CHAIN_ADMIN_ADDR,
        OSMOSIS_CHAIN_DENOM,
        OSMOSIS_CHAIN_PREFIX,
        LOCAL_CODE_ID_CACHE_PATH_OSMOSIS,
        "neutron-osmosis",
        salt,
        &authorization_contract_address,
    )?;
    info!("processor on osmosis: {:?}", processor_on_osmosis);

    let current_dir = env::current_dir()?;

    upload_contracts(current_dir, &mut test_ctx)?;

    let (osmo_gamm_lper_addr, osmo_input_acc_addr, osmo_output_acc_addr, pool_id) =
        osmosis_setup(&mut test_ctx, &processor_on_osmosis)?;

    let (
        broker_addr,
        asserter_addr,
        neutron_storage_account,
        ic_querier_addr,
        ibc_forwarder,
        neutron_forwarder_input_acc,
    ) = neutron_setup(
        &mut test_ctx,
        &neutron_processor_address,
        pool_id,
        &osmo_input_acc_addr,
    )?;

    info!("creating authorizations...");
    create_authorizations(
        &mut test_ctx,
        &authorization_contract_address,
        ic_querier_addr.to_string(),
        asserter_addr,
        osmo_gamm_lper_addr,
        ibc_forwarder,
    )?;

    // REGISTERING KV QUERY

    info!("sending kv query registration message to authorizations");
    let kv_query_registration_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                FunctionMsgs::RegisterKvQuery {
                    target_query: TARGET_QUERY_LABEL.to_string(),
                },
            ),
        )?),
    };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "register_kv_query".to_string(),
            messages: vec![kv_query_registration_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&send_msg).unwrap(),
        &format!("{GAS_FLAGS} --fees=100000untrn"),
    )?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor on neutron to register the KV query...");
    let kvq_tick_response = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &neutron_processor_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )?,
        "--gas=auto --gas-adjustment=5.0 --fees=5000000untrn",
    )?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Check processor queues...");
    let items = get_processor_queue_items(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &neutron_processor_address,
        Priority::Medium,
    );
    let osmo_items = get_processor_queue_items(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &processor_on_osmosis,
        Priority::Medium,
    );
    println!("Items on neutron processor: {:?}", items);
    println!("Items on osmosis processor: {:?}", osmo_items);

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    info!("routing LP subroutine instructions to osmosis processsor...");
    let lp_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_osmosis_gamm_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                    expected_spot_price: None,
                    asset: ntrn_on_osmo_denom,
                    limit: Uint128::new(100_000_000),
                },
            ),
        )?),
    };
    let provide_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "provide_liquidity".to_string(),
            messages: vec![lp_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&provide_liquidity_msg)?,
        GAS_FLAGS,
    )?;

    info!("confirming that osmosis processor enqueued the provide_liquidity_msg...");
    confirm_remote_domain_processor_queue_length(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &processor_on_osmosis,
        1,
    );

    let assertion_message_binary = Binary::from(serde_json::to_vec(
        &valence_middleware_asserter::msg::ExecuteMsg::Assert {
            cfg: AssertionConfig {
                a: valence_middleware_asserter::msg::AssertionValue::Variable(
                    valence_middleware_asserter::msg::QueryInfo {
                        storage_account: neutron_storage_account.to_string(),
                        storage_slot_key: TARGET_QUERY_LABEL.to_string(),
                        query: to_json_binary(&XykPoolQuery::GetPrice {})?,
                    },
                ),
                predicate: valence_middleware_asserter::msg::Predicate::LT,
                b: valence_middleware_asserter::msg::AssertionValue::Constant(
                    valence_middleware_utils::type_registry::queries::ValencePrimitive::Decimal(
                        Decimal::from_str("20.0").unwrap(),
                    ),
                ),
            },
        },
    )?);

    let assertion_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: assertion_message_binary,
    };
    let ibc_transfer_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_neutron_ibc_transfer_library::msg::FunctionMsgs::IbcTransfer {},
            ),
        )?),
    };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "conditional_lp_authorization".to_string(),
            messages: vec![assertion_message, ibc_transfer_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&send_msg).unwrap(),
        &format!("{GAS_FLAGS} --fees=1000000untrn"),
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Check processor queue");
    let items = get_processor_queue_items(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        &neutron_processor_address,
        Priority::Medium,
    );
    info!("Items on neutron processor: {:?}", items);
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor on neutron to perform the assertion & forward the funds to osmosis input account...");
    let ibc_forward_tick_response = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &neutron_processor_address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        "--gas=auto --gas-adjustment=5.0 --fees=50000000untrn",
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(5));
    info!("ibc_forward_tick_response: {:?}", ibc_forward_tick_response);

    let query_processor_callbacks_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &authorization_contract_address,
            &serde_json::to_string(
                &valence_authorization_utils::msg::QueryMsg::ProcessorCallbacks {
                    start_after: None,
                    limit: None,
                },
            )?,
        )["data"]
            .clone(),
    )?;

    info!(
        "{NEUTRON_CHAIN_NAME} authorization mod processor callbacks: {:?}",
        query_processor_callbacks_response
    );
    std::thread::sleep(std::time::Duration::from_secs(3));

    let osmo_input_acc_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_input_acc_addr,
    );
    let osmo_output_acc_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_output_acc_addr,
    );

    info!("osmo_input_acc_balance: {:?}", osmo_input_acc_balance);
    info!("osmo_output_acc_balance: {:?}", osmo_output_acc_balance);

    info!("confirming that osmosis processor enqueued the provide_liquidity_msg...");
    confirm_remote_domain_processor_queue_length(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        &processor_on_osmosis,
        1,
    );

    info!("Ticking processor on osmosis...");
    let osmosis_lp_tick_response = contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &processor_on_osmosis,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        "--gas=auto --gas-adjustment=5.0 --fees=5000000uosmo",
    )
    .unwrap();

    info!(
        "osmo lp tick response: {:?}",
        osmosis_lp_tick_response.tx_hash.unwrap()
    );
    std::thread::sleep(std::time::Duration::from_secs(3));

    let osmo_input_acc_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_input_acc_addr,
    );
    let osmo_output_acc_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        &osmo_output_acc_addr,
    );

    info!("osmo_input_acc_balance: {:?}", osmo_input_acc_balance);
    info!("osmo_output_acc_balance: {:?}", osmo_output_acc_balance);
    Ok(())
}

fn osmosis_setup(
    mut test_ctx: &mut TestContext,
    processor_on_osmosis: &str,
) -> Result<(String, String, String, u64), Box<dyn Error>> {
    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();
    let pool_id = setup_gamm_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    let current_dir: std::path::PathBuf = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );
    let gamm_lper_contract_path = format!(
        "{}/artifacts/valence_osmosis_gamm_lper.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&base_account_contract_path)?;
    uploader
        .with_chain_name(OSMOSIS_CHAIN_NAME)
        .send_single_contract(&gamm_lper_contract_path)?;

    let osmosis_base_acc_code_id = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    let osmo_base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        OSMOSIS_CHAIN_NAME,
        osmosis_base_acc_code_id,
        OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        vec![processor_on_osmosis.to_string()],
        2,
        Some(Coin::new(1000000u128, OSMOSIS_CHAIN_DENOM)),
    );
    let osmo_input_acc_addr = osmo_base_accounts.first().unwrap();
    let osmo_output_acc_addr = osmo_base_accounts.get(1).unwrap();
    info!("osmo_input_acc_addr: {osmo_input_acc_addr}");
    info!("osmo_output_acc_addr: {osmo_output_acc_addr}");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let osmo_gamm_lper_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_osmosis_gamm_lper::msg::LibraryConfig,
    > {
        owner: OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        processor: processor_on_osmosis.to_string(),
        config: valence_osmosis_gamm_lper::msg::LibraryConfig::new(
            LibraryAccountType::Addr(osmo_input_acc_addr.to_string()),
            LibraryAccountType::Addr(osmo_output_acc_addr.to_string()),
            valence_osmosis_gamm_lper::msg::LiquidityProviderConfig {
                pool_id,
                asset_data: valence_library_utils::liquidity_utils::AssetData {
                    asset1: OSMOSIS_CHAIN_DENOM.to_string(),
                    asset2: ntrn_on_osmo_denom.to_string(),
                },
            },
        ),
    };

    let gamm_lper_code_id = test_ctx
        .get_contract()
        .src(OSMOSIS_CHAIN_NAME)
        .contract("valence_osmosis_gamm_lper")
        .get_cw()
        .code_id
        .unwrap();

    let osmo_gamm_lper_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        DEFAULT_KEY,
        gamm_lper_code_id,
        &serde_json::to_string(&osmo_gamm_lper_instantiate_msg)?,
        "osmo_gamm_lper_lib",
        Some(OSMOSIS_CHAIN_ADMIN_ADDR),
        &format!("{GAS_FLAGS} --fees=100000uosmo"),
    )?;

    let osmo_gamm_lper_addr = osmo_gamm_lper_lib.address.to_string();

    std::thread::sleep(std::time::Duration::from_secs(1));
    info!("osmo_gamm_lper_addr: {osmo_gamm_lper_addr}");

    info!("approving gamm lper lib on the osmo input acc");
    approve_library(
        &mut test_ctx,
        OSMOSIS_CHAIN_NAME,
        DEFAULT_KEY,
        osmo_input_acc_addr,
        osmo_gamm_lper_addr.to_string(),
        Some(format!("{GAS_FLAGS} --fees=100000uosmo")),
    );

    Ok((
        osmo_gamm_lper_addr,
        osmo_input_acc_addr.to_string(),
        osmo_output_acc_addr.to_string(),
        pool_id,
    ))
}

fn neutron_setup(
    mut test_ctx: &mut TestContext,
    neutron_processor_address: &str,
    pool_id: u64,
    osmo_input_acc: &str,
) -> Result<(String, String, String, String, String, String), Box<dyn Error>> {
    let current_dir = env::current_dir()?;
    let ntrn_to_osmo_connection_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    // with test context set up, we can generate the .env file for the icq relayer
    generate_icq_relayer_config(
        &test_ctx,
        current_dir.clone(),
        OSMOSIS_CHAIN_NAME.to_string(),
    )?;

    // start the icq relayer. this runs in detached mode so we need
    // to manually kill it before each run for now.
    start_icq_relayer()?;

    info!("sleeping for 10 to allow icq relayer to start...");
    std::thread::sleep(Duration::from_secs(10));

    let (broker_addr, asserter_addr, _) = setup_middleware(&mut test_ctx)?;

    // set up the storage account
    info!("setting up storage accounts...");
    let storage_acc_code_id = test_ctx
        .get_contract()
        .contract("valence_storage_account")
        .get_cw()
        .code_id
        .unwrap();

    let storage_accounts = create_storage_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        storage_acc_code_id,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![
            neutron_processor_address.to_string(),
            NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        ],
        1,
        None,
    );
    let neutron_storage_account = storage_accounts.first().unwrap();
    info!("neutron storage account address: {neutron_storage_account}",);

    // set up the IC querier
    let neutron_ic_querier_lib_code_id = test_ctx
        .get_contract()
        .contract("valence_neutron_ic_querier")
        .get_cw()
        .code_id
        .unwrap();

    let icq_lib_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<LibraryConfig> {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: neutron_processor_address.to_string(),
        config: LibraryConfig::new(
            LibraryAccountType::Addr(neutron_storage_account.to_string()),
            valence_neutron_ic_querier::msg::QuerierConfig {
                broker_addr: broker_addr.to_string(),
                connection_id: ntrn_to_osmo_connection_id,
            },
            BTreeMap::from_iter(vec![(
                TARGET_QUERY_LABEL.to_string(),
                QueryDefinition {
                    registry_version: None,
                    type_url: osmosis_std::types::osmosis::gamm::v1beta1::Pool::TYPE_URL
                        .to_string(),
                    update_period: Uint64::new(5),
                    params: BTreeMap::from([(
                        "pool_id".to_string(),
                        to_json_binary(&pool_id).unwrap(),
                    )]),
                    query_id: None,
                },
            )]),
        ),
    };
    let ic_querier_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ic_querier_lib_code_id,
        &serde_json::to_string(&icq_lib_instantiate_msg)?,
        "icq_querier_lib",
        Some(NEUTRON_CHAIN_ADMIN_ADDR),
        "",
    )?;
    std::thread::sleep(std::time::Duration::from_secs(1));
    info!("ic_querier lib address: {}", ic_querier_lib.address);

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &ic_querier_lib.address.to_string(),
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: 1_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("approving IC querier lib on the storage account");
    approve_library(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        neutron_storage_account,
        ic_querier_lib.address.to_string(),
        None,
    );

    let neutron_base_acc_code_id = test_ctx
        .get_contract()
        .src(NEUTRON_CHAIN_NAME)
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    let neutron_base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        neutron_base_acc_code_id,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![neutron_processor_address.to_string()],
        1,
        Some(Coin::new(1000000u128, NEUTRON_CHAIN_DENOM)),
    );
    let neutron_input_acc_addr = neutron_base_accounts.first().unwrap();

    // Get the code id
    let code_id_ibc_transfer_lib = test_ctx
        .get_contract()
        .contract("valence_neutron_ibc_transfer_library")
        .get_cw()
        .code_id
        .unwrap();

    info!("Creating IBC transfer library contract");
    let transfer_amount = 100_000_000u128;
    let ntrn_osmo_path = &(
        NEUTRON_CHAIN_NAME.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
    );
    let ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_neutron_ibc_transfer_library::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: neutron_processor_address.to_string(),
        config: valence_neutron_ibc_transfer_library::msg::LibraryConfig::new(
            LibraryAccountType::Addr(neutron_input_acc_addr.clone()),
            osmo_input_acc.to_string(),
            UncheckedDenom::Native(NTRN_DENOM.to_string()),
            IbcTransferAmount::FixedAmount(transfer_amount.into()),
            "".to_owned(),
            valence_neutron_ibc_transfer_library::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .transfer_channel_ids
                    .get(ntrn_osmo_path)
                    .unwrap()
                    .clone(),
                ibc_transfer_timeout: Some(600u64.into()),
            },
        ),
    };
    info!(
        "IBC Transfer instantiate message: {:?}",
        ibc_transfer_instantiate_msg
    );

    let ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_ibc_transfer_lib,
        &serde_json::to_string(&ibc_transfer_instantiate_msg).unwrap(),
        "ibc_transfer",
        None,
        "",
    )
    .unwrap();

    info!("IBC Transfer library: {}", ibc_transfer.address.clone());

    // Approve the library for the base account
    approve_library(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &neutron_input_acc_addr,
        ibc_transfer.address.clone(),
        None,
    );

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &neutron_input_acc_addr.to_string(),
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: 100_000_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok((
        broker_addr,
        asserter_addr,
        neutron_storage_account.to_string(),
        ic_querier_lib.address.to_string(),
        ibc_transfer.address,
        neutron_input_acc_addr.to_string(),
    ))
}

fn create_authorizations(
    test_ctx: &mut TestContext,
    authorization_contract_address: &str,
    ic_querier: String,
    asserter: String,
    gamm_lper: String,
    ibc_forwarder: String,
) -> Result<(), Box<dyn Error>> {
    let register_kvq_authorization = AuthorizationBuilder::new()
        .with_label("register_kv_query")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(ic_querier.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let deregister_kvq_authorization = AuthorizationBuilder::new()
        .with_label("deregister_kv_query")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(ic_querier.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let conditional_lp_authorization = AuthorizationBuilder::new()
        .with_label("conditional_lp_authorization")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(asserter.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "assert".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(ibc_forwarder))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let osmosis_lp_authorization = AuthorizationBuilder::new()
        .with_label("provide_liquidity")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
                        .with_contract_address(LibraryAccountType::Addr(gamm_lper))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_function".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let authorizations = vec![
        register_kvq_authorization,
        deregister_kvq_authorization,
        conditional_lp_authorization,
        osmosis_lp_authorization,
    ];

    info!("Creating execute authorization...");
    let create_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations { authorizations },
    );

    std::thread::sleep(std::time::Duration::from_secs(3));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorization_contract_address.to_string().as_str(),
        DEFAULT_KEY,
        &serde_json::to_string(&create_authorization).unwrap(),
        &format!("{GAS_FLAGS} --fees=100000untrn"),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let query_authorizations_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            authorization_contract_address,
            &serde_json::to_string(
                &valence_authorization_utils::msg::QueryMsg::Authorizations {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();
    info!(
        "authorizations created: {:?}",
        query_authorizations_response.as_array().unwrap()
    );
    let authorizations = query_authorizations_response.as_array().unwrap();

    assert!(authorizations.len() == 4);

    info!("Authorizations created!");

    Ok(())
}

pub fn set_type_registry(
    test_ctx: &TestContext,
    broker: String,
    type_registry_addr: String,
    type_registry_version: String,
) -> Result<TransactionResponse, LocalError> {
    let set_registry_msg = valence_middleware_broker::msg::ExecuteMsg::SetRegistry {
        version: type_registry_version.to_string(),
        address: type_registry_addr,
    };

    let stringified_msg = serde_json::to_string(&set_registry_msg)
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?;

    info!("registering type registry v.{type_registry_version}");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &broker,
        DEFAULT_KEY,
        &stringified_msg,
        "--amount 1000000untrn --gas 50000000",
    )
}

pub fn register_kvq(
    test_ctx: &TestContext,
    icq_lib: String,
    target_query: String,
) -> Result<TransactionResponse, LocalError> {
    let register_kvq_fn = FunctionMsgs::RegisterKvQuery { target_query };

    let tx_execute_msg =
        valence_library_utils::msg::ExecuteMsg::<FunctionMsgs, ()>::ProcessFunction(
            register_kvq_fn,
        );

    let stringified_msg = serde_json::to_string(&tx_execute_msg)
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?;

    info!(
        "registering ICQ KV query on querier {icq_lib} :  {:?}",
        stringified_msg
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        DEFAULT_KEY,
        &stringified_msg,
        "--amount 1000000untrn --gas 50000000",
    )
}

pub fn deregister_kvq(
    test_ctx: &TestContext,
    icq_lib: String,
    target_query: String,
) -> Result<TransactionResponse, LocalError> {
    let deregister_kvq_fn = FunctionMsgs::DeregisterKvQuery { target_query };

    let tx_execute_msg =
        valence_library_utils::msg::ExecuteMsg::<FunctionMsgs, ()>::ProcessFunction(
            deregister_kvq_fn,
        );

    let stringified_msg = serde_json::to_string(&tx_execute_msg)
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?;

    info!(
        "deregistering ICQ KV query from querier {icq_lib} :  {:?}",
        stringified_msg
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        DEFAULT_KEY,
        &stringified_msg,
        "--gas 50000000",
    )
}

pub fn query_storage_account(
    test_ctx: &TestContext,
    storage_account: String,
    storage_key: String,
) -> Result<ValenceType, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &storage_account,
        &serde_json::to_string(&valence_storage_account::msg::QueryMsg::QueryValenceType {
            key: storage_key,
        })
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    info!("query response: {:?}", query_response);
    let resp: Result<ValenceType, serde_json::error::Error> =
        serde_json::from_value(query_response);

    match resp {
        Ok(val) => Ok(val),
        Err(e) => Err(LocalError::Custom { msg: e.to_string() }),
    }
}

pub fn broker_get_canonical(
    test_ctx: &TestContext,
    broker_addr: String,
    type_url: String,
    binary: Binary,
) -> Result<ValenceType, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &broker_addr,
        &serde_json::to_string(&valence_middleware_broker::msg::QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::ToCanonical { type_url, binary },
        })
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    info!("query response: {:?}", query_response);
    let resp: ValenceType = serde_json::from_value(query_response).unwrap();

    Ok(resp)
}

fn upload_contracts(
    current_dir: PathBuf,
    test_ctx: &mut TestContext,
) -> Result<(), Box<dyn Error>> {
    info!("uploading contracts to neutron & osmosis...");
    let mut uploader = test_ctx.build_tx_upload_contracts();
    let osmosis_type_registry_middleware_path = format!(
        "{}/artifacts/valence_middleware_osmosis.wasm",
        current_dir.display()
    );
    let osmosis_middleware_broker_path = format!(
        "{}/artifacts/valence_middleware_broker.wasm",
        current_dir.display()
    );
    let icq_lib_local_path = format!(
        "{}/artifacts/valence_neutron_ic_querier.wasm",
        current_dir.display()
    );
    let storage_acc_path = format!(
        "{}/artifacts/valence_storage_account.wasm",
        current_dir.display()
    );
    let base_acc_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );
    let asserter_path = format!(
        "{}/artifacts/valence_middleware_asserter.wasm",
        current_dir.display()
    );
    let neutron_ibc_forwarder_path = format!(
        "{}/artifacts/valence_neutron_ibc_transfer_library.wasm",
        current_dir.display()
    );

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&icq_lib_local_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&osmosis_type_registry_middleware_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&osmosis_middleware_broker_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&storage_acc_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&asserter_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&neutron_ibc_forwarder_path)?;
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&base_acc_path)?;

    Ok(())
}

fn setup_middleware(
    test_ctx: &mut TestContext,
) -> Result<(String, String, String), Box<dyn Error>> {
    info!("setting up the middleware...");
    let type_registry_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_osmosis")
        .get_cw()
        .code_id
        .unwrap();
    let asserter_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_asserter")
        .get_cw()
        .code_id
        .unwrap();
    let broker_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_broker")
        .get_cw()
        .code_id
        .unwrap();

    let type_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        type_registry_code_id,
        &serde_json::to_string(&RegistryInstantiateMsg {})?,
        "type_registry",
        None,
        "",
    )?;
    info!(
        "type_registry_contract address: {}",
        type_registry_contract.address
    );
    std::thread::sleep(Duration::from_secs(1));
    let asserter_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        asserter_code_id,
        &serde_json::to_string(&valence_middleware_asserter::msg::InstantiateMsg {})?,
        "asserter",
        None,
        "",
    )?;

    info!("asserter_contract address: {}", asserter_contract.address);
    std::thread::sleep(Duration::from_secs(1));
    let broker_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        broker_code_id,
        &serde_json::to_string(&valence_middleware_broker::msg::InstantiateMsg {})?,
        "broker",
        None,
        "",
    )?;
    info!("middleware broker address: {}", broker_contract.address);
    std::thread::sleep(Duration::from_secs(1));

    let resp = set_type_registry(
        test_ctx,
        broker_contract.address.to_string(),
        type_registry_contract.address.to_string(),
        "26.0.0".to_string(),
    )?;
    std::thread::sleep(Duration::from_secs(2));
    info!("added type registry response: {:?}", resp.tx_hash.unwrap());

    Ok((
        broker_contract.address,
        asserter_contract.address,
        type_registry_contract.address,
    ))
}

pub fn approve_library(
    test_ctx: &mut TestContext,
    chain_name: &str,
    key: &str,
    base_account: &str,
    library: String,
    flags: Option<String>,
) {
    let approve_msg = valence_account_utils::msg::ExecuteMsg::ApproveLibrary {
        library: library.clone(),
    };
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(chain_name),
        base_account,
        key,
        &serde_json::to_string(&approve_msg).unwrap(),
        &format!(
            "{}{}",
            GAS_FLAGS,
            flags
                .map(|mut s| {
                    if !s.starts_with(' ') {
                        s.insert(0, ' ');
                    }
                    s
                })
                .unwrap_or_default()
        ),
    )
    .unwrap();

    info!(
        "Approved library {} for base account {}",
        library, base_account
    );
    std::thread::sleep(std::time::Duration::from_secs(2));
}

pub fn get_processor_queue_items(
    test_ctx: &mut TestContext,
    chain_name: &str,
    processor_address: &str,
    priority: Priority,
) -> Vec<MessageBatch> {
    serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(chain_name),
            processor_address,
            &serde_json::to_string(&valence_processor_utils::msg::QueryMsg::GetQueue {
                from: None,
                to: None,
                priority,
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap()
}
