use std::{fs::File, io::Write, path::PathBuf};

use localic_utils::{utils::test_context::TestContext, NEUTRON_CHAIN_NAME};

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
        "tcp://local{}-1-val-0-neutron_gaia_junoic:26657",
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
        neutron_rpc = "tcp://localneutron-1-val-0-neutron_gaia_junoic:26657",
        neutron_rest = "http://localneutron-1-val-0-neutron_gaia_junoic:1317",
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
