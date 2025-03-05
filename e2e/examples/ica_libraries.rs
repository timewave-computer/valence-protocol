use std::error::Error;

use localic_utils::{
    types::config::ConfigChain, ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_NAME,
};
use valence_chain_client_utils::noble::NobleClient;
use valence_e2e::utils::{
    file::get_grpc_address_and_port, noble::set_up_noble, ADMIN_MNEMONIC, LOGS_FILE_PATH,
    NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME,
    NOBLE_CHAIN_PREFIX, VALENCE_ARTIFACTS_PATH,
};

const UUSDC_DENOM: &str = "uusdc";

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let mut _test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_chain(ConfigChain {
            denom: NOBLE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: NOBLE_CHAIN_ID.to_string(),
            chain_name: NOBLE_CHAIN_NAME.to_string(),
            chain_prefix: NOBLE_CHAIN_PREFIX.to_string(),
            admin_addr: NOBLE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, NOBLE_CHAIN_NAME)
        .build()?;

    let rt = tokio::runtime::Runtime::new()?;
    // Get the grpc url and the port for the noble chain
    let (grpc_url, grpc_port) = get_grpc_address_and_port(NOBLE_CHAIN_ID)?;

    let noble_client = rt.block_on(async {
        NobleClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NOBLE_CHAIN_ID,
            NOBLE_CHAIN_DENOM,
        )
        .await
        .unwrap()
    });

    // Set up our noble environment to allow for testing
    rt.block_on(set_up_noble(&noble_client, 0, UUSDC_DENOM));

    Ok(())
}
