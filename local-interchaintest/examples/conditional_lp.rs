use cosmwasm_std::{to_json_binary, Binary, Uint64};
use cosmwasm_std_old::to_json_string;
use local_interchaintest::utils::{
    authorization::{set_up_authorization_and_processor, set_up_external_domain_with_polytone},
    base_account::create_storage_accounts,
    icq::{generate_icq_relayer_config, start_icq_relayer},
    osmosis::gamm::setup_gamm_pool,
    GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_OSMOSIS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query},
    types::TransactionResponse,
};
use log::info;
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    path::PathBuf,
    time::{Duration, SystemTime},
};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_library_utils::LibraryAccountType;
use valence_middleware_asserter::msg::AssertionConfig;
use valence_middleware_utils::type_registry::types::{
    RegistryInstantiateMsg, RegistryQueryMsg, ValenceType,
};
use valence_neutron_ic_querier::msg::{FunctionMsgs, LibraryConfig, QueryDefinition};

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

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();
    let ntrn_to_osmo_connection_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let pool_id = setup_gamm_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    let current_dir = env::current_dir()?;

    // // with test context set up, we can generate the .env file for the icq relayer
    // generate_icq_relayer_config(
    //     &test_ctx,
    //     current_dir.clone(),
    //     OSMOSIS_CHAIN_NAME.to_string(),
    // )?;

    // // start the icq relayer. this runs in detached mode so we need
    // // to manually kill it before each run for now.
    // start_icq_relayer()?;

    // info!("sleeping for 10 to allow icq relayer to start...");
    // std::thread::sleep(Duration::from_secs(10));

    let now = SystemTime::now();
    let b64_seconds = to_json_string(
        &now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    )?;
    let salt = hex::encode(b64_seconds);
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
    upload_contracts(current_dir, &mut test_ctx)?;

    let (broker_addr, asserter_addr, type_registry_addr) = setup_middleware(&mut test_ctx)?;

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
        vec![neutron_processor_address.clone()],
        1,
        None,
    );
    let neutron_storage_account = storage_accounts.first().unwrap();
    info!(
        "neutron storage account address: {:?}",
        neutron_storage_account
    );

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
    let icq_test_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ic_querier_lib_code_id,
        &serde_json::to_string(&icq_lib_instantiate_msg)?,
        "icq_querier_lib",
        None,
        "",
    )?;
    info!("icq querier lib address: {}", icq_test_lib.address);

    create_authorizations(
        neutron_storage_account.to_string(),
        &mut test_ctx,
        &authorization_contract_address,
    )?;

    Ok(())
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
    let asserter_path = format!(
        "{}/artifacts/valence_middleware_asserter.wasm",
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
    std::thread::sleep(Duration::from_secs(3));
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
    std::thread::sleep(Duration::from_secs(3));
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
    std::thread::sleep(Duration::from_secs(3));

    set_type_registry(
        test_ctx,
        broker_contract.address.to_string(),
        type_registry_contract.address.to_string(),
        "26.0.0".to_string(),
    )?;
    std::thread::sleep(Duration::from_secs(2));

    Ok((
        broker_contract.address,
        asserter_contract.address,
        type_registry_contract.address,
    ))
}

fn create_authorizations(
    storage_account: String,
    test_ctx: &mut TestContext,
    authorization_contract_address: &str,
) -> Result<(), Box<dyn Error>> {
    let register_kvq_authorization = AuthorizationBuilder::new()
        .with_label("register")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(storage_account.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "register_kvq".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let deregister_kvq_authorization = AuthorizationBuilder::new()
        .with_label("deregister")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(storage_account.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "deregister_kvq".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    let assertion_authorization = AuthorizationBuilder::new()
        .with_label("assert")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_domain(valence_authorization_utils::domain::Domain::Main)
                        .with_contract_address(LibraryAccountType::Addr(storage_account.clone()))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "assert_msg".to_string(),
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
        assertion_authorization,
    ];

    info!("Creating execute authorization...");
    let create_authorization = valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
        valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations { authorizations },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorization_contract_address.to_string().as_str(),
        DEFAULT_KEY,
        &serde_json::to_string(&create_authorization).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Authorizations created!");
    Ok(())
}

fn send_msgs_to_authorization_contract(
    storage_account: String,
    test_ctx: &mut TestContext,
    authorization_contract_address: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Send the messages to the authorization contract...");

    // let register_kvq_fn = FunctionMsgs::RegisterKvQuery { target_query };

    // let tx_execute_msg =
    //     valence_library_utils::msg::ExecuteMsg::<FunctionMsgs, ()>::ProcessFunction(
    //         register_kvq_fn,
    //     );

    // let stringified_msg = serde_json::to_string(&tx_execute_msg)
    //     .map_err(|e| LocalError::Custom { msg: e.to_string() })?;

    // let msg_liquid_stake = MsgLiquidStake {
    //     amount: Some(Coin {
    //         denom: neutron_on_persistence.clone(),
    //         amount: amount_to_liquid_stake.to_string(),
    //     }),
    //     delegator_address: persistence_base_account.clone(),
    // };
    // #[allow(deprecated)]
    // let liquid_staking_message = CosmosMsg::Stargate {
    //     type_url: msg_liquid_stake.to_any().type_url,
    //     value: Binary::from(msg_liquid_stake.to_proto_bytes()),
    // };

    // let binary = Binary::from(
    //     serde_json::to_vec(&valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
    //         msgs: vec![liquid_staking_message],
    //     })
    //     .unwrap(),
    // );
    // let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    // let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
    //     valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
    //         label: "execute".to_string(),
    //         messages: vec![message],
    //         ttl: None,
    //     },
    // );

    // contract_execute(
    //     test_ctx
    //         .get_request_builder()
    //         .get_request_builder(NEUTRON_CHAIN_NAME),
    //     &authorization_contract_address,
    //     DEFAULT_KEY,
    //     &serde_json::to_string(&send_msg).unwrap(),
    //     GAS_FLAGS,
    // )
    // .unwrap();
    // std::thread::sleep(std::time::Duration::from_secs(3));
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

pub fn assert_predicate(
    test_ctx: &TestContext,
    asserter: String,
    assertion_cfg: AssertionConfig,
) -> Result<String, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &asserter,
        &serde_json::to_string(&valence_middleware_asserter::msg::QueryMsg::Assert(
            assertion_cfg,
        ))
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    let resp: Result<String, serde_json::error::Error> = serde_json::from_value(query_response);

    match resp {
        Ok(val) => Ok(val),
        Err(e) => Err(LocalError::Custom { msg: e.to_string() }),
    }
}
