use std::error::Error;

use local_interchaintest::utils::{
    persistence::{activate_host_zone, register_host_zone},
    LOGS_FILE_PATH, PERSISTENCE_CHAIN_ADMIN_ADDR, PERSISTENCE_CHAIN_DENOM, PERSISTENCE_CHAIN_ID,
    PERSISTENCE_CHAIN_NAME, PERSISTENCE_CHAIN_PREFIX, VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{
    types::config::ConfigChain, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
};
use log::info;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChain {
            denom: PERSISTENCE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: PERSISTENCE_CHAIN_ID.to_string(),
            chain_name: PERSISTENCE_CHAIN_NAME.to_string(),
            chain_prefix: PERSISTENCE_CHAIN_PREFIX.to_string(),
            admin_addr: PERSISTENCE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, PERSISTENCE_CHAIN_NAME)
        .build()?;

    let channel_id = test_ctx
        .get_transfer_channels()
        .src(PERSISTENCE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let connection_id = test_ctx
        .get_connections()
        .src(PERSISTENCE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let native_denom = test_ctx.get_native_denom().src(NEUTRON_CHAIN_NAME).get();

    info!("Registering host zone...");
    register_host_zone(
        test_ctx
            .get_request_builder()
            .get_request_builder(PERSISTENCE_CHAIN_NAME),
        NEUTRON_CHAIN_ID,
        &connection_id,
        &channel_id,
        &native_denom,
        DEFAULT_KEY,
    )?;

    info!("Activating host zone...");
    activate_host_zone(NEUTRON_CHAIN_ID)?;

    Ok(())
}
