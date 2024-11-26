use local_interchaintest::utils::{
    icq::{
        generate_icq_relayer_config, query_catchall_logs, query_raw_result,
        register_kvq_balances_query, start_icq_relayer, try_parse_storage_value,
    },
    GAS_FLAGS, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_std::{
    errors::LocalError,
    modules::cosmwasm::{contract_execute, contract_instantiate, contract_query},
    types::TransactionResponse,
};
use log::info;
use neutron_sdk::{
    bindings::types::{InterchainQueryResult, StorageValue},
    interchain_queries::{
        helpers::decode_and_convert,
        v047::{helpers::create_account_denom_balance_key, types::BANK_STORE_KEY},
    },
};
use serde_json::Value;
use std::{env, error::Error, time::Duration};
use valence_test_icq_lib::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_ADMIN_ADDR, OSMOSIS_CHAIN_DENOM,
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

    // let icq_registration_response = register_icq_balances_query(
    //     &test_ctx,
    //     icq_test_lib.address.to_string(),
    //     OSMOSIS_CHAIN_NAME.to_string(),
    //     OSMOSIS_CHAIN_ADMIN_ADDR.to_string(),
    //     vec![OSMOSIS_CHAIN_DENOM.to_string()],
    // )?;

    // info!("icq registration response: {:?}", icq_registration_response);

    // for _ in 0..10 {
    //     let response_value =
    //         query_balance_query_id(&test_ctx, icq_test_lib.address.to_string(), 1)?;

    //     info!("response value: {:?}", response_value);

    //     if !response_value.balances.coins.is_empty() {
    //         break;
    //     } else {
    //         std::thread::sleep(Duration::from_secs(5));
    //     }
    // }

    std::thread::sleep(Duration::from_secs(3));
    let addr = "osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63";

    let converted_addr_bytes = decode_and_convert(&addr).unwrap();

    let balance_key = create_account_denom_balance_key(converted_addr_bytes, "uosmo").unwrap();

    let kvq_registration_response = register_kvq_balances_query(
        &test_ctx,
        icq_test_lib.address.to_string(),
        OSMOSIS_CHAIN_NAME.to_string(),
        BANK_STORE_KEY.to_string(),
        balance_key,
    )?;

    info!(
        "kv query registration response: {:?}",
        kvq_registration_response
    );

    for _ in 0..2 {
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
