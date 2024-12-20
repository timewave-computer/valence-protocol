use cosmwasm_std::{to_json_binary, Binary};
use local_interchaintest::utils::{
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
use valence_icq_querier::msg::FunctionMsgs;
use valence_middleware_utils::type_registry::types::RegistryInstantiateMsg;

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_DENOM,
    OSMOSIS_CHAIN_NAME,
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
        "{}/artifacts/valence_icq_querier.wasm",
        current_dir.display()
    );

    info!("sleeping for 5 to allow icq relayer to start...");
    std::thread::sleep(Duration::from_secs(10));

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&icq_lib_local_path)?;
    std::thread::sleep(Duration::from_secs(1));
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&osmosis_type_registry_middleware_path)?;
    std::thread::sleep(Duration::from_secs(1));
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&osmosis_middleware_broker_path)?;
    std::thread::sleep(Duration::from_secs(1));

    // set up the ICQ querier
    let icq_querier_lib_code_id = test_ctx
        .get_contract()
        .contract("valence_icq_querier")
        .get_cw()
        .code_id
        .unwrap();
    let icq_test_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        icq_querier_lib_code_id,
        &serde_json::to_string(&valence_icq_querier::msg::InstantiateMsg {})?,
        "icq_querier_lib",
        None,
        "",
    )?;
    info!("icq querier lib address: {}", icq_test_lib.address);

    std::thread::sleep(Duration::from_secs(3));

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

    // associate type registry with broker
    let set_registry_response = set_type_registry(
        &test_ctx,
        broker_contract.address.to_string(),
        type_registry_contract.address,
        "26.0.0".to_string(),
    )?;

    info!(
        "type registry addition response: {:?}",
        set_registry_response
    );
    std::thread::sleep(Duration::from_secs(2));

    let gamm_query_params =
        BTreeMap::from([("pool_id".to_string(), to_json_binary(&pool_id).unwrap())]);

    let ntrn_to_osmo_connection_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    let kvq_registration_response = register_kvq(
        &test_ctx,
        icq_test_lib.address.to_string(),
        broker_contract.address.to_string(),
        osmosis_std::types::osmosis::gamm::v1beta1::Pool::TYPE_URL.to_string(),
        ntrn_to_osmo_connection_id,
        gamm_query_params,
    )?;

    info!(
        "KEY VALUE QUERY REGISTRATION RESPONSE TXHASH: {}",
        kvq_registration_response.tx_hash.unwrap()
    );

    std::thread::sleep(Duration::from_secs(10));

    info!("querying results...");
    let query_results_response = query_results(&test_ctx, icq_test_lib.address.to_string())?;

    info!("query results: {:?}", query_results_response);

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
    broker_addr: String,
    type_id: String,
    connection_id: String,
    params: BTreeMap<String, Binary>,
) -> Result<TransactionResponse, LocalError> {
    let register_kvq_msg = FunctionMsgs::RegisterKvQuery {
        broker_addr,
        type_id,
        connection_id,
        params,
    };

    let stringified_msg = serde_json::to_string(&register_kvq_msg)
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
) -> Result<Vec<(u64, Binary)>, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&valence_icq_querier::msg::QueryMsg::QueryResults {})
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    println!("query response: {:?}", query_response);
    let resp: Vec<(u64, Binary)> = serde_json::from_value(query_response).unwrap();

    Ok(resp)
}
