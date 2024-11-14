use std::error::Error;

use local_interchaintest::utils::{
    manager::{setup_manager, OSMOSIS_GAMM_LPER_NAME, OSMOSIS_GAMM_LWER_NAME},
    LOGS_FILE_PATH, NEUTRON_OSMO_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};

use localic_std::modules::bank;
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_ADMIN_ADDR,
    OSMOSIS_CHAIN_NAME,
};
use log::info;
use valence_program_manager::program_config_builder::ProgramConfigBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_osmosis().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, OSMOSIS_CHAIN_NAME)
        .build()?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_OSMO_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![OSMOSIS_GAMM_LPER_NAME, OSMOSIS_GAMM_LWER_NAME],
    )?;

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());
    let osmo_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(OSMOSIS_CHAIN_NAME.to_string());

    let ntrn_on_osmo_denom = test_ctx
        .get_ibc_denom()
        .base_denom(NEUTRON_CHAIN_DENOM.to_owned())
        .src(NEUTRON_CHAIN_NAME)
        .dest(OSMOSIS_CHAIN_NAME)
        .get();

    info!("transferring 1000 neutron tokens to osmo admin addr for pool creation...");
    test_ctx
        .build_tx_transfer()
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .with_amount(1_000_000_000u128)
        .with_recipient(OSMOSIS_CHAIN_ADMIN_ADDR)
        .with_denom(NEUTRON_CHAIN_DENOM)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(OSMOSIS_CHAIN_NAME),
        OSMOSIS_CHAIN_ADMIN_ADDR,
    );
    info!("osmosis chain admin addr balances: {:?}", token_balances);

    test_ctx
        .build_tx_create_osmo_pool()
        .with_weight("uosmo", 1)
        .with_weight(&ntrn_on_osmo_denom, 1)
        .with_initial_deposit("uosmo", 1)
        .with_initial_deposit(&ntrn_on_osmo_denom, 1)
        .send()?;

    // Get its id
    let pool_id = test_ctx
        .get_osmo_pool()
        .denoms("uosmo".into(), ntrn_on_osmo_denom)
        .get_u64();

    info!("Gamm pool id: {:?}", pool_id);

    Ok(())
}
