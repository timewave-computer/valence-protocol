use localic_std::modules::cosmwasm::contract_instantiate;
use log::info;
use std::{env, error::Error, time::Duration};
use valence_e2e::utils::{
    icq::{
        generate_icq_relayer_config, query_catchall_logs, register_kvq_balances_query,
        start_icq_relayer,
    },
    osmosis::gamm::setup_gamm_pool,
    LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use valence_test_icq_lib::msg::InstantiateMsg;

use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_DENOM, OSMOSIS_CHAIN_NAME,
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
    let icq_test_lib_local_path = format!(
        "{}/artifacts/valence_test_icq_lib.wasm",
        current_dir.display()
    );

    info!("sleeping to allow icq relayer to start...");
    std::thread::sleep(Duration::from_secs(10));

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&icq_test_lib_local_path)?;

    let icq_test_lib_code_id = test_ctx
        .get_contract()
        .contract("valence_test_icq_lib")
        .get_cw()
        .code_id
        .unwrap();

    info!("icq test lib code id: {icq_test_lib_code_id}");

    // instantiate icq test lib
    let icq_test_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        icq_test_lib_code_id,
        &serde_json::to_string(&InstantiateMsg {})?,
        "valence_test_icq_lib",
        None,
        "",
    )?;

    info!("icq test lib: {}", icq_test_lib.address);

    info!("attempting GAMM total liquidity query");

    let mut total_liquidity_key = vec![PREFIX_POOLS_KEY];
    total_liquidity_key.extend_from_slice(&pool_id.to_be_bytes());

    info!("base64 liquidity key: {:?}", total_liquidity_key);

    let kvq_registration_response = register_kvq_balances_query(
        &test_ctx,
        icq_test_lib.address.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
        GAMM_STORE_KEY.to_string(),
        total_liquidity_key,
    )?;

    info!(
        "kv query registration response: {:?}",
        kvq_registration_response
    );

    std::thread::sleep(Duration::from_secs(5));

    let catchall_logs = query_catchall_logs(&test_ctx, icq_test_lib.address.to_string())?;
    info!("catchall logs: {:?}", catchall_logs);

    Ok(())
}
