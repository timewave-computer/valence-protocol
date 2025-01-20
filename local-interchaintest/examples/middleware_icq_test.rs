use cosmwasm_std::{to_json_binary, Binary, Uint64};
use local_interchaintest::utils::{
    base_account::approve_library,
    icq::{generate_icq_relayer_config, start_icq_relayer},
    osmosis::gamm::setup_gamm_pool,
    LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query},
    types::TransactionResponse,
};
use log::info;
use std::{collections::BTreeMap, env, error::Error, time::Duration};
use valence_library_utils::LibraryAccountType;
use valence_middleware_utils::type_registry::types::{
    RegistryInstantiateMsg, RegistryQueryMsg, ValenceType,
};
use valence_neutron_ic_querier::msg::{FunctionMsgs, LibraryConfig};

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_NAME,
};

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

    // fire the query registration request
    let icq_registration_resp = register_kvq(
        &test_ctx,
        icq_test_lib.address.to_string(),
        osmosis_std::types::osmosis::gamm::v1beta1::Pool::TYPE_URL.to_string(),
        Uint64::new(5),
        BTreeMap::from([("pool_id".to_string(), to_json_binary(&pool_id).unwrap())]),
    )?;

    info!(
        "icq registration response: {:?}",
        icq_registration_resp.tx_hash.unwrap()
    );

    std::thread::sleep(Duration::from_secs(10));

    info!("querying results...");
    let query_results_response = query_results(&test_ctx, icq_test_lib.address.to_string())?;

    info!("query results: {:?}", query_results_response);

    let hopefully_valence_type = query_results_response[0].1.clone();

    info!("valence xyk pool: {:?}", hopefully_valence_type);

    match hopefully_valence_type {
        ValenceType::XykPool(valence_xyk_pool) => {
            let price = valence_xyk_pool.get_price();
            info!("price: {:?}", price);
        }
        _ => panic!("should be xyk pool"),
    };

    let storage_account_value = query_storage_account(
        &test_ctx,
        storage_acc_contract.address,
        "test_result".to_string(),
    )?;

    info!("storage account value: {:?}", storage_account_value);

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
    type_id: String,
    update_period: Uint64,
    params: BTreeMap<String, Binary>,
) -> Result<TransactionResponse, LocalError> {
    let register_kvq_fn = FunctionMsgs::RegisterKvQuery {
        registry_version: None,
        type_id,
        update_period,
        params,
    };

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

pub fn query_results(
    test_ctx: &TestContext,
    icq_lib: String,
) -> Result<Vec<(u64, ValenceType)>, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&valence_neutron_ic_querier::msg::QueryMsg::QueryResults {})
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    info!("query response: {:?}", query_response);
    let resp: Vec<(u64, ValenceType)> = serde_json::from_value(query_response).unwrap();

    Ok(resp)
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
