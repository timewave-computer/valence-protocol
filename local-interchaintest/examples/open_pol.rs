use std::{env, error::Error, time::SystemTime};

use local_interchaintest::utils::{
    authorization::set_up_authorization_and_processor, manager::setup_manager, LOGS_FILE_PATH,
    NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // setup_manager(
    //     &mut test_ctx,
    //     NEUTRON_CONFIG_FILE,
    //     vec![GAIA_CHAIN_NAME],
    //     vec![SPLITTER_NAME],
    // )?;

    // Let's upload the base account contract to Neutron
    let current_dir = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&base_account_contract_path)?;

    let now = SystemTime::now();
    let salt = hex::encode(
        now.duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );

    let (authorization_contract_address, _) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    // Now that we have the processor on persistence, let's create a base account and approve it
    let current_dir: std::path::PathBuf = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );

    Ok(())
}
