use std::{fs::File, io::Write, path::PathBuf};

use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_query},
    types::TransactionResponse,
};
use localic_utils::{utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_NAME};
use log::info;
use neutron_sdk::bindings::types::InterchainQueryResult;
use valence_test_icq_lib::msg::{ExecuteMsg, QueryMsg};

pub fn generate_icq_relayer_config(
    test_ctx: &TestContext,
    current_path: PathBuf,
    target_domain: String,
) -> std::io::Result<()> {
    let target_connection_id = test_ctx
        .get_connections()
        .src(NEUTRON_CHAIN_NAME)
        .dest(&target_domain)
        .get();

    // formatted according to neutron ICQ relayer docs
    let target_chain_rpc = format!(
        "tcp://local{}-1-val-0-neutron_osmosisic:26657",
        target_domain
    );
    let env_content = format!(
        r#"
RELAYER_NEUTRON_CHAIN_RPC_ADDR={neutron_rpc}
RELAYER_NEUTRON_CHAIN_REST_ADDR={neutron_rest}
RELAYER_NEUTRON_CHAIN_HOME_DIR=/data
RELAYER_NEUTRON_CHAIN_SIGN_KEY_NAME=acc3
RELAYER_NEUTRON_CHAIN_GAS_PRICES=0.5untrn
RELAYER_NEUTRON_CHAIN_GAS_LIMIT=10000000
RELAYER_NEUTRON_CHAIN_GAS_ADJUSTMENT=1.3
RELAYER_NEUTRON_CHAIN_DENOM=untrn
RELAYER_NEUTRON_CHAIN_MAX_GAS_PRICE=1000
RELAYER_NEUTRON_CHAIN_GAS_PRICE_MULTIPLIER=3.0
RELAYER_NEUTRON_CHAIN_CONNECTION_ID={connection_id}
RELAYER_NEUTRON_CHAIN_DEBUG=true
RELAYER_NEUTRON_CHAIN_KEYRING_BACKEND=test
RELAYER_NEUTRON_CHAIN_ACCOUNT_PREFIX=neutron
RELAYER_NEUTRON_CHAIN_KEY=acc3
RELAYER_NEUTRON_CHAIN_OUTPUT_FORMAT=json
RELAYER_NEUTRON_CHAIN_SIGN_MODE_STR=direct

RELAYER_TARGET_CHAIN_RPC_ADDR={target_rpc}
RELAYER_TARGET_CHAIN_TIMEOUT=10s
RELAYER_TARGET_CHAIN_DEBUG=true
RELAYER_TARGET_CHAIN_KEYRING_BACKEND=test
RELAYER_TARGET_CHAIN_OUTPUT_FORMAT=json

RELAYER_REGISTRY_ADDRESSES=
RELAYER_REGISTRY_QUERY_IDS=

RELAYER_ALLOW_TX_QUERIES=true
RELAYER_ALLOW_KV_CALLBACKS=true
RELAYER_STORAGE_PATH=storage/leveldb
RELAYER_WEBSERVER_PORT=127.0.0.1:9999
RELAYER_IGNORE_ERRORS_REGEX=(execute wasm contract failed|failed to build tx query string)
"#,
        neutron_rpc = "tcp://localneutron-1-val-0-neutron_osmosisic:26657",
        neutron_rest = "http://localneutron-1-val-0-neutron_osmosisic:1317",
        connection_id = target_connection_id,
        target_rpc = target_chain_rpc,
    );

    // create the env file and write the dynamically generated config there
    let path = current_path
        .join("local-interchaintest")
        .join("configs")
        .join(".env");
    let mut file = File::create(path)?;
    file.write_all(env_content.as_bytes())?;

    Ok(())
}

pub fn start_icq_relayer() -> Result<(), Box<dyn std::error::Error>> {
    // try to stop and remove any existing icq-relayer container from host
    let _ = std::process::Command::new("docker")
        .arg("stop")
        .arg("icq-relayer")
        .output();
    let _ = std::process::Command::new("docker")
        .arg("rm")
        .arg("icq-relayer")
        .output();

    let output = std::process::Command::new("docker")
        .arg("inspect")
        .arg("localneutron-1-val-0-neutron_osmosisic")
        .output()
        .expect("failed to inspect the neutron container");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON from docker inspect output");

    // extract the docker network under which neutron container is spinning
    let docker_network = response[0]["NetworkSettings"]["Networks"].clone();
    let network_name = docker_network
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .to_string();

    // extract the mount point of neutron chain on host machine
    let mount_point = response[0]["Mounts"][0]["Source"].as_str().unwrap();

    // this should be initiated by `just local-ic-run`, so we know the relpath
    let env_relpath = "local-interchaintest/configs/.env";

    let start_icq_relayer_cmd = std::process::Command::new("docker")
        .arg("run")
        .arg("-d") // detached mode to not block the main()
        .arg("--name")
        .arg("icq-relayer")
        .arg("--env-file")
        .arg(env_relpath) // passing the .env file we generated before
        .arg("-p")
        .arg("9999:9999") // the port binding for the relayer webserver, idk if it's needed
        .arg("--network")
        .arg(network_name) // docker network under which we want to run the relayer
        .arg("-v")
        .arg(format!("{}:/data", mount_point)) // neutron mount point to access the keyring
        .arg("neutron-org/neutron-query-relayer")
        .output()
        .expect("failed to start icq relayer");

    if start_icq_relayer_cmd.status.success() {
        let container_id = String::from_utf8_lossy(&start_icq_relayer_cmd.stdout)
            .trim()
            .to_string();
        info!("ICQ relayer container started with ID: {container_id}");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&start_icq_relayer_cmd.stderr);
        Err(format!("Failed to start ICQ relayer: {error}").into())
    }
}

pub fn register_kvq_balances_query(
    test_ctx: &TestContext,
    icq_lib: String,
    domain: String,
    path: String,
    key: Vec<u8>,
) -> Result<TransactionResponse, LocalError> {
    info!("registering ICQ KV query on domain {domain}...");

    let register_kvq_msg = ExecuteMsg::RegisterKeyValueQuery {
        connection_id: test_ctx
            .get_connections()
            .src(NEUTRON_CHAIN_NAME)
            .dest(&domain)
            .get(),
        update_period: 5,
        path,
        key,
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
