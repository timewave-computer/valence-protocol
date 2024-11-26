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
use neutron_sdk::bindings::types::{InterchainQueryResult, StorageValue};
use serde_json::Value;
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

        if !response_value.balances.coins.is_empty() {
            break;
        } else {
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    std::thread::sleep(Duration::from_secs(3));
    let kvq_registration_response = register_kvq_balances_query(
        &test_ctx,
        icq_test_lib.address.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
        OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
    )?;

    info!(
        "kv query registration response: {:?}",
        kvq_registration_response
    );

    for _ in 0..10 {
        let response_value = query_catchall_logs(&test_ctx, icq_test_lib.address.to_string())?;
        info!("catchall logs: {:?}", response_value);

        let raw_query_resp = query_raw_result(&test_ctx, icq_test_lib.address.to_string(), 1)?;
        info!("raw query response: {:?}", raw_query_resp);

        for kv_result in raw_query_resp.kv_results {
            // let key = kv_result.key;
            // let value = kv_result.value;

            let parse_attempt = try_parse_storage_value(&kv_result);

            info!("\nPARSE ATTEMPT: {:?}", parse_attempt);
        }

        if !response_value.is_empty() {
            break;
        } else {
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    Ok(())
}

fn try_parse_storage_value(storage_value: &StorageValue) -> Value {
    let mut map = serde_json::Map::new();

    // Add storage prefix
    map.insert(
        "storage_prefix".to_string(),
        Value::String(storage_value.storage_prefix.clone()),
    );

    // Try UTF-8 string interpretation
    if let Ok(key_str) = String::from_utf8(storage_value.key.to_vec()) {
        map.insert("key_utf8".to_string(), Value::String(key_str));
    }

    if let Ok(value_str) = String::from_utf8(storage_value.value.to_vec()) {
        map.insert("value_utf8".to_string(), Value::String(value_str));
    }

    // Try JSON interpretation
    if let Ok(value_str) = String::from_utf8(storage_value.value.to_vec()) {
        if let Ok(json_value) = serde_json::from_str(&value_str) {
            map.insert("value_json".to_string(), json_value);
        }
    }

    if let Ok(value_str) = String::from_utf8(storage_value.key.to_vec()) {
        if let Ok(json_value) = serde_json::from_str(&value_str) {
            map.insert("key_json".to_string(), json_value);
        }
    }

    // Convert raw bytes to base64
    map.insert(
        "key".to_string(),
        Value::String(storage_value.key.to_string()),
    );

    Value::Object(map)
}

fn register_kvq_balances_query(
    test_ctx: &TestContext,
    icq_lib: String,
    domain: String,
    addr: String,
) -> Result<TransactionResponse, LocalError> {
    info!("registering ICQ KV query on domain {domain}...");

    let register_kvq_msg = ExecuteMsg::RegisterKeyValueQuery {
        connection_id: test_ctx
            .get_connections()
            .src(NEUTRON_CHAIN_NAME)
            .dest(&domain)
            .get(),
        update_period: 5,
        key: addr,
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

pub fn query_raw_result(
    test_ctx: &TestContext,
    icq_lib: String,
    query_id: u64,
) -> Result<InterchainQueryResult, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&QueryMsg::RawIcqResult { id: query_id })
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    let icq_result: InterchainQueryResult = serde_json::from_value(query_response).unwrap();

    info!("raw icq result: {:?}", icq_result);

    Ok(icq_result)
}

pub fn query_catchall_logs(
    test_ctx: &TestContext,
    icq_lib: String,
) -> Result<Vec<(String, String)>, LocalError> {
    let query_response = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &icq_lib,
        &serde_json::to_string(&QueryMsg::Catchall {})
            .map_err(|e| LocalError::Custom { msg: e.to_string() })?,
    )["data"]
        .clone();

    let resp: Vec<(String, String)> = serde_json::from_value(query_response).unwrap();

    Ok(resp)
}
