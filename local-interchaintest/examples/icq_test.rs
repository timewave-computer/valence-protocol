use local_interchaintest::utils::{
    icq::{generate_icq_relayer_config, start_icq_relayer},
    GAS_FLAGS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query},
    types::TransactionResponse,
};
use log::info;
use std::{env, error::Error, time::Duration};
use valence_test_icq_lib::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM,
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

    let icq_lib_instantiate_msg = InstantiateMsg {};

    // instantiate icq test lib
    let icq_test_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        icq_test_lib_code_id,
        &serde_json::to_string(&icq_lib_instantiate_msg)?,
        "valence_test_icq_lib",
        None,
        "",
    )?;

    info!("icq test lib: {}", icq_test_lib.address);

    let icq_registration_response = register_icq_balances_query(
        &test_ctx,
        icq_test_lib.address.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
        OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
        vec![OSMOSIS_CHAIN_DENOM.to_string()],
    )?;

    info!("icq registration response: {:?}", icq_registration_response);

    for _ in 0..10 {
        let response_value =
            query_balance_query_id(&test_ctx, icq_test_lib.address.to_string(), 1)?;

        info!("response value: {:?}", response_value);
        std::thread::sleep(Duration::from_secs(5));
    }

    Ok(())
}

pub fn register_icq_balances_query(
    test_ctx: &TestContext,
    icq_lib: String,
    domain: String,
    addr: String,
    denoms: Vec<String>,
) -> Result<TransactionResponse, LocalError> {
    info!("registering ICQ balances query on domain {domain} for {addr}...");

    let register_icq_msg = ExecuteMsg::RegisterBalancesQuery {
        connection_id: test_ctx
            .get_connections()
            .src(NEUTRON_CHAIN_NAME)
            .dest(&domain)
            .get(),
        update_period: 5,
        addr,
        denoms,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        DEFAULT_KEY,
        &serde_json::to_string(&register_icq_msg)
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
        "--amount 1000000untrn --gas 50000000",
    )
}

pub fn query_balance_query_id(
    test_ctx: &TestContext,
    icq_lib: String,
    query_id: u64,
) -> Result<neutron_sdk::interchain_queries::v047::queries::BalanceResponse, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&QueryMsg::Balance { query_id })
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    let balance_response: neutron_sdk::interchain_queries::v047::queries::BalanceResponse =
        serde_json::from_value(query_response).unwrap();

    info!("balance query response: {:?}", balance_response);

    Ok(balance_response)
}
