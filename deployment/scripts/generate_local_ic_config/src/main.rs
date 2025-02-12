use clap::{arg, command, Parser};
use std::{error::Error, fs};

use local_interchaintest::utils::{
    manager::{
        get_global_config, setup_manager, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME,
        FORWARDER_NAME, GENERIC_IBC_TRANSFER_NAME, NEUTRON_IBC_TRANSFER_NAME, OSMOSIS_CL_LPER_NAME,
        OSMOSIS_CL_LWER_NAME, OSMOSIS_GAMM_LPER_NAME, OSMOSIS_GAMM_LWER_NAME, POLYTONE_NOTE_NAME,
        POLYTONE_PROXY_NAME, POLYTONE_VOICE_NAME, REVERSE_SPLITTER_NAME, SPLITTER_NAME,
    },
    NEUTRON_CONFIG_FILE,
};
use localic_utils::{ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL};

pub const LOGS_FILE_PATH: &str = "local-interchaintest/configs/logs.json";
pub const VALENCE_ARTIFACTS_PATH: &str = "artifacts";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enviroment config to use
    #[arg(short, long)]
    chain_config: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // Setup local enviroment
    setup_manager(
        &mut test_ctx,
        args.chain_config
            .unwrap_or(NEUTRON_CONFIG_FILE.to_string())
            .as_str(),
        vec![GAIA_CHAIN_NAME],
        vec![
            SPLITTER_NAME,
            // REVERSE_SPLITTER_NAME,
            // FORWARDER_NAME,
            // GENERIC_IBC_TRANSFER_NAME,
            // NEUTRON_IBC_TRANSFER_NAME,
            // ASTROPORT_LPER_NAME,
            // ASTROPORT_WITHDRAWER_NAME,
            // OSMOSIS_GAMM_LPER_NAME,
            // OSMOSIS_GAMM_LWER_NAME,
            // OSMOSIS_CL_LPER_NAME,
            // OSMOSIS_CL_LWER_NAME,
            // POLYTONE_NOTE_NAME,
            // POLYTONE_VOICE_NAME,
            // POLYTONE_PROXY_NAME,
        ],
    )?;

    // Export manager config to file
    exprt_manager_config()?;

    Ok(())
}

fn exprt_manager_config() -> Result<(), Box<dyn Error>> {
    let gc = get_global_config();

    let t = toml::to_string(&*gc).unwrap();
    fs::write("deployment/configs/local/config.toml", t)?;
    Ok(())
}
