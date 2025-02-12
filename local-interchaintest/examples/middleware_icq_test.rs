use cosmwasm_std::{to_json_binary, Binary, Decimal, Uint64};
use local_interchaintest::utils::{
    base_account::approve_library,
    icq::{generate_icq_relayer_config, start_icq_relayer},
    osmosis::gamm::setup_gamm_pool,
    LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
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
use std::{collections::BTreeMap, env, error::Error, str::FromStr, time::Duration};
use valence_library_utils::LibraryAccountType;
use valence_middleware_asserter::msg::{AssertionValue, Predicate, QueryInfo};
use valence_middleware_utils::{
    canonical_types::pools::xyk::XykPoolQuery,
    type_registry::{
        queries::{ValencePrimitive, ValenceTypeQuery},
        types::{RegistryInstantiateMsg, RegistryQueryMsg, ValenceType},
    },
};
use valence_neutron_ic_querier::msg::{Config, FunctionMsgs, LibraryConfig, QueryDefinition};

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_NAME,
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
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let pool_id = setup_gamm_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

    let current_dir = env::current_dir()?;

    // with test context set up, we can generate the .env file for the icq relayer
    generate_icq_relayer_config(
        &test_ctx,
        current_dir.clone(),
        OSMOSIS_CHAIN_NAME.to_string(),
    )?;

    // start the icq relayer. this runs in detached mode so we need
    // to manually kill it before each run for now.
    start_icq_relayer()?;

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

    info!("sleeping for 10 to allow icq relayer to start...");
    std::thread::sleep(Duration::from_secs(10));

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

    let ntrn_to_osmo_connection_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    // set up the type registry
    let type_registry_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_osmosis")
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

    // set up the asserter
    let asserter_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_asserter")
        .get_cw()
        .code_id
        .unwrap();
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

    // set up the broker
    let broker_code_id = test_ctx
        .get_contract()
        .contract("valence_middleware_broker")
        .get_cw()
        .code_id
        .unwrap();
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

    // set up the storage account
    let storage_acc_code_id = test_ctx
        .get_contract()
        .contract("valence_storage_account")
        .get_cw()
        .code_id
        .unwrap();

    let storage_acc_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        storage_acc_code_id,
        &serde_json::to_string(&valence_account_utils::msg::InstantiateMsg {
            admin: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
            approved_libraries: vec![],
        })?,
        "storage_account",
        None,
        "",
    )?;

    info!("storage account: {}", storage_acc_contract.address);
    std::thread::sleep(Duration::from_secs(3));

    // set up the IC querier
    let neutron_ic_querier_lib_code_id = test_ctx
        .get_contract()
        .contract("valence_neutron_ic_querier")
        .get_cw()
        .code_id
        .unwrap();

    let query_definitions = BTreeMap::from_iter(vec![(
        TARGET_QUERY_LABEL.to_string(),
        QueryDefinition {
            registry_version: None,
            type_url: osmosis_std::types::osmosis::gamm::v1beta1::Pool::TYPE_URL.to_string(),
            update_period: Uint64::new(5),
            params: BTreeMap::from([("pool_id".to_string(), to_json_binary(&pool_id).unwrap())]),
            query_id: None,
        },
    )]);
    let icq_test_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ic_querier_lib_code_id,
        &serde_json::to_string(
            &valence_library_utils::msg::InstantiateMsg::<LibraryConfig> {
                owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
                processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
                config: LibraryConfig::new(
                    LibraryAccountType::Addr(storage_acc_contract.address.to_string()),
                    valence_neutron_ic_querier::msg::QuerierConfig {
                        broker_addr: broker_contract.address.to_string(),
                        connection_id: ntrn_to_osmo_connection_id,
                    },
                    query_definitions,
                ),
            },
        )?,
        "icq_querier_lib",
        None,
        "",
    )?;
    info!("icq querier lib address: {}", icq_test_lib.address);

    std::thread::sleep(Duration::from_secs(3));

    info!(
        "approving icq test lib {} on storage account {}",
        icq_test_lib.address, storage_acc_contract.address
    );
    approve_library(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        storage_acc_contract.address.as_str(),
        icq_test_lib.address.to_string(),
        None,
    );
    std::thread::sleep(Duration::from_secs(3));

    // associate type registry with broker
    set_type_registry(
        &test_ctx,
        broker_contract.address.to_string(),
        type_registry_contract.address,
        "26.0.0".to_string(),
    )?;
    std::thread::sleep(Duration::from_secs(2));

    let icq_lib_config = query_library_config(&test_ctx, icq_test_lib.address.to_string())?;
    assert!(icq_lib_config.pending_query_registrations.is_empty());
    assert!(icq_lib_config.registered_queries.is_empty());
    assert!(icq_lib_config.query_definitions[TARGET_QUERY_LABEL]
        .query_id
        .is_none());

    // fire the query registration request
    let icq_registration_resp = register_kvq(
        &test_ctx,
        icq_test_lib.address.to_string(),
        TARGET_QUERY_LABEL.to_string(),
    )?;

    info!(
        "icq registration response: {:?}",
        icq_registration_resp.tx_hash.clone().unwrap()
    );

    std::thread::sleep(Duration::from_secs(10));

    info!("querying results...");
    let storage_account_value = query_storage_account(
        &test_ctx,
        storage_acc_contract.address.to_string(),
        TARGET_QUERY_LABEL.to_string(),
    )?;

    info!("storage account value: {:?}", storage_account_value);

    let icq_lib_config = query_library_config(&test_ctx, icq_test_lib.address.to_string())?;
    assert!(icq_lib_config.pending_query_registrations.is_empty());
    assert!(icq_lib_config.registered_queries.len() == 1);
    assert!(icq_lib_config.query_definitions[TARGET_QUERY_LABEL]
        .query_id
        .is_some());

    match storage_account_value {
        ValenceType::XykPool(valence_xyk_pool) => {
            let query_msg = to_json_binary(&XykPoolQuery::GetPrice {}).unwrap();
            let price = valence_xyk_pool.query(query_msg).unwrap();
            info!("price: {:?}", price);
        }
        _ => panic!("should be xyk pool"),
    };

    std::thread::sleep(Duration::from_secs(2));

    info!("deregistering the kv query #{TARGET_QUERY_LABEL}");

    let pre_admin_token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        NEUTRON_CHAIN_ADMIN_ADDR,
    );
    info!("pre_admin_token_balances: {:?}", pre_admin_token_balances);

    let deregistration_response = deregister_kvq(
        &test_ctx,
        icq_test_lib.address.to_string(),
        TARGET_QUERY_LABEL.to_string(),
    )?;

    info!(
        "query deregistration tx hash: {:?}",
        deregistration_response.tx_hash.unwrap()
    );

    std::thread::sleep(Duration::from_secs(2));

    // after deregistration the config should reflect the changes
    let icq_lib_config = query_library_config(&test_ctx, icq_test_lib.address.to_string())?;
    assert!(icq_lib_config.pending_query_registrations.is_empty());
    assert!(icq_lib_config.registered_queries.is_empty());
    assert!(icq_lib_config.query_definitions[TARGET_QUERY_LABEL]
        .query_id
        .is_none());

    let post_admin_token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        NEUTRON_CHAIN_ADMIN_ADDR,
    );
    info!("post_admin_token_balances: {:?}", post_admin_token_balances);

    let pre_deregistration_ntrn_bal = pre_admin_token_balances
        .iter()
        .find(|b| b.denom == NEUTRON_CHAIN_DENOM)
        .unwrap()
        .amount;
    let post_deregistration_ntrn_bal = post_admin_token_balances
        .iter()
        .find(|b| b.denom == NEUTRON_CHAIN_DENOM)
        .unwrap()
        .amount;

    // assert that the admin was credited with icq registration
    // escrow refund
    assert_eq!(
        post_deregistration_ntrn_bal
            .checked_sub(pre_deregistration_ntrn_bal)
            .unwrap()
            .u128(),
        1000000
    );

    info!("asserting with the asserter. storage account slot price: 0.833");
    info!("assert (slot < 0.9) ?..");
    let resp = assert_predicate(
        &test_ctx,
        asserter_contract.address.to_string(),
        AssertionValue::Variable(QueryInfo {
            storage_account: storage_acc_contract.address.to_string(),
            storage_slot_key: "gamm_pool".to_string(),
            query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
        }),
        Predicate::LT,
        AssertionValue::Constant(ValencePrimitive::Decimal(Decimal::from_str("0.9").unwrap())),
    );
    match resp {
        Ok(val) => info!("success: {:?}", val),
        Err(e) => info!("error: {:?}", e),
    }

    info!("assert (slot > 0.9) ?..");
    let resp = assert_predicate(
        &test_ctx,
        asserter_contract.address,
        AssertionValue::Variable(QueryInfo {
            storage_account: storage_acc_contract.address.to_string(),
            storage_slot_key: "gamm_pool".to_string(),
            query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
        }),
        Predicate::GT,
        AssertionValue::Constant(ValencePrimitive::Decimal(Decimal::from_str("0.9").unwrap())),
    );
    match resp {
        Ok(val) => info!("success: {:?}", val),
        Err(e) => info!("error: {:?}", e),
    }
    Ok(())
}

pub fn extract_registered_icq_id(
    test_ctx: &mut TestContext,
    tx_hash: String,
) -> Result<u64, Box<dyn Error>> {
    let registered_query_response = test_ctx
        .get_request_builder()
        .get_request_builder(NEUTRON_CHAIN_NAME)
        .query_tx_hash(&tx_hash)["events"]
        .clone();

    let query_registration_attribute = registered_query_response
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| match e["attributes"].clone() {
            serde_json::Value::Array(vec) => Some(vec),
            _ => None,
        })
        .flatten()
        .find(|e| e["key"] == "query_id")
        .unwrap();

    let query_id_str = match query_registration_attribute["value"].clone() {
        serde_json::Value::String(n) => n.to_string(),
        _ => panic!("query_id not found in icq registration response"),
    };

    let query_id: u64 = query_id_str.parse().unwrap();

    info!("registered query id: #{query_id}");

    Ok(query_id)
}

pub fn query_library_config(test_ctx: &TestContext, icq_lib: String) -> Result<Config, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&valence_neutron_ic_querier::msg::QueryMsg::GetLibraryConfig {})
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    info!("query response: {:?}", query_response);
    let resp: Result<Config, serde_json::error::Error> = serde_json::from_value(query_response);

    match resp {
        Ok(val) => Ok(val),
        Err(e) => Err(LocalError::Custom { msg: e.to_string() }),
    }
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
    a: AssertionValue,
    predicate: Predicate,
    b: AssertionValue,
) -> Result<TransactionResponse, LocalError> {
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &asserter,
        DEFAULT_KEY,
        &serde_json::to_string(&valence_middleware_asserter::msg::ExecuteMsg::Assert {
            a,
            predicate,
            b,
        })
        .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
        "",
    )
}
