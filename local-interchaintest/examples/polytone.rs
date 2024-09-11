use std::{env, error::Error, path::Path};

use localic_std::modules::cosmwasm::CosmWasm;
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, JUNO_CHAIN_NAME,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_NAME,
};
use valence_local_interchaintest_utils::constants::{
    LOCAL_CODE_ID_CACHE_PATH_JUNO, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, POLYTONE_PATH,
    VALENCE_PATH,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir("artifacts")
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChainBuilder::default_gaia().build()?)
        .with_chain(ConfigChainBuilder::default_juno().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, GAIA_CHAIN_NAME)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, JUNO_CHAIN_NAME)
        .build()?;

    let mut uploader = test_ctx.build_tx_upload_contracts();

    // Upload all Polytone contracts to both Neutron and Juno
    uploader
        .send_with_local_cache(
            POLYTONE_PATH,
            NEUTRON_CHAIN_NAME,
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )
        .unwrap();

    uploader
        .send_with_local_cache(
            POLYTONE_PATH,
            JUNO_CHAIN_NAME,
            LOCAL_CODE_ID_CACHE_PATH_JUNO,
        )
        .unwrap();

    // Upload the authorization contract to Neutron and the processor to both Neutron and Juno
    let authorization_contract_path = format!("{}/valence_authorization.wasm", VALENCE_PATH);
    let processor_contract_path = format!("{}/valence_processor.wasm", VALENCE_PATH);
    let current_dir = env::current_dir()?;

    let mut cw = CosmWasm::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
    );
    let authorization_contract_code_id = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(authorization_contract_path).as_path())?,
        )
        .unwrap();

    let processor_contract_code_id_on_neutron = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(processor_contract_path.clone()).as_path())?,
        )
        .unwrap();

    let mut cw = CosmWasm::new(
        test_ctx
            .get_request_builder()
            .get_request_builder(JUNO_CHAIN_NAME),
    );

    let processor_contract_code_id_on_juno = cw
        .store(
            DEFAULT_KEY,
            &Path::canonicalize(current_dir.join(processor_contract_path).as_path())?,
        )
        .unwrap();

    Ok(())
}
