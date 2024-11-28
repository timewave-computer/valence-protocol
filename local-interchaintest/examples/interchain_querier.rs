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
use std::{env, error::Error, time::Duration};
use valence_icq_querier::msg::{FunctionMsgs, InstantiateMsg, QueryMsg};

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_DENOM,
    OSMOSIS_CHAIN_NAME,
};

// KeyNextGlobalPoolId defines key to store the next Pool ID to be used.
pub const NEXT_GLOBAL_POOL_ID_KEY: u8 = 0x01;
pub const PREFIX_POOLS_KEY: u8 = 0x02;
pub const TOTAL_LIQUIDITY_KEY: u8 = 0x03;
pub const PREFIX_MIGRATION_INFO_BALANCER_POOL_KEY: u8 = 0x04;
pub const GAMM_STORE_KEY: &str = "gamm";

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

    let _pool_id = setup_gamm_pool(&mut test_ctx, OSMOSIS_CHAIN_DENOM, &ntrn_on_osmo_denom)?;

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
    let icq_lib_local_path = format!(
        "{}/artifacts/valence_icq_querier.wasm",
        current_dir.display()
    );

    info!("sleeping to allow icq relayer to start...");
    std::thread::sleep(Duration::from_secs(10));

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&icq_lib_local_path)?;

    let icq_querier_lib_code_id = test_ctx
        .get_contract()
        .contract("valence_icq_querier")
        .get_cw()
        .code_id
        .unwrap();

    info!("icq querier library code id: {icq_querier_lib_code_id}");

    // instantiate icq querier lib
    let icq_test_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        icq_querier_lib_code_id,
        &serde_json::to_string(&InstantiateMsg {})?,
        "icq_querier_lib",
        None,
        "",
    )?;

    info!("icq querier lib address: {}", icq_test_lib.address);

    info!("attempting GAMM total liquidity query");

    let kvq_registration_response = register_kvq(
        &test_ctx,
        icq_test_lib.address.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
        "gamm".to_string(),
    )?;

    info!(
        "kv query registration response: {:?}",
        kvq_registration_response
    );

    std::thread::sleep(Duration::from_secs(5));

    let kvq_registration_response = register_kvq(
        &test_ctx,
        icq_test_lib.address.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
        "bank".to_string(),
    )?;

    info!(
        "kv query registration response: {:?}",
        kvq_registration_response
    );

    std::thread::sleep(Duration::from_secs(5));

    let logs = query_logs(&test_ctx, icq_test_lib.address.to_string())?;
    info!("logs: {:?}", logs);

    let results = query_results(&test_ctx, icq_test_lib.address.to_string())?;
    info!("results: {:?}", results);

    Ok(())
}

pub fn register_kvq(
    test_ctx: &TestContext,
    icq_lib: String,
    domain: String,
    module: String,
) -> Result<TransactionResponse, LocalError> {
    info!("registering ICQ KV query on domain {domain} for mod {module}...");

    let register_kvq_msg = FunctionMsgs::RegisterKvQuery {
        connection_id: test_ctx
            .get_connections()
            .src(NEUTRON_CHAIN_NAME)
            .dest(&domain)
            .get(),
        update_period: 5,
        module,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        DEFAULT_KEY,
        &serde_json::to_string(&register_kvq_msg)
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
        "--amount 1000000untrn --gas 50000000",
    )
}

pub fn query_logs(
    test_ctx: &TestContext,
    icq_lib: String,
) -> Result<Vec<(String, String)>, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&QueryMsg::Logs {})
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    let resp: Vec<(String, String)> = serde_json::from_value(query_response).unwrap();

    Ok(resp)
}

pub fn query_results(
    test_ctx: &TestContext,
    icq_lib: String,
) -> Result<Vec<(u64, String)>, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&QueryMsg::QueryResults {})
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    let resp: Vec<(u64, String)> = serde_json::from_value(query_response).unwrap();

    Ok(resp)
}
