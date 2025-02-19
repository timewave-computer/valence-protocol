use std::{env, error::Error, fs};

use localic_utils::{ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL};
use valence_e2e::utils::{
    manager::{get_global_config, setup_manager, SPLITTER_NAME},
    LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![SPLITTER_NAME],
    )?;

    let gc = get_global_config();

    // Save the config to a file
    let curr_path = env::current_dir()?;
    let config_dev_path = format!(
        "{}/e2e/configs/config_dev.toml",
        curr_path.to_str().unwrap()
    );

    let t = toml::to_string(&*gc).unwrap();
    fs::write(config_dev_path, t)?;

    Ok(())
}
