use std::{env, error::Error};

use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, JUNO_CHAIN_NAME,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_local_interchaintest_utils::constants::{
    LOCAL_CODE_ID_CACHE_PATH_JUNO, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, POLYTONE_PATH,
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
        .send_with_local_cache(POLYTONE_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)
        .unwrap();

    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_with_local_cache(POLYTONE_PATH, LOCAL_CODE_ID_CACHE_PATH_JUNO)
        .unwrap();

    // Upload the authorization contract to Neutron and the processor to both Neutron and Juno
    let current_dir = env::current_dir()?;

    let authorization_contract_path = format!(
        "{}/artifacts/valence_authorization.wasm",
        current_dir.display()
    );

    info!("{}", authorization_contract_path);

    let processor_contract_path =
        format!("{}/artifacts/valence_processor.wasm", current_dir.display());
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_single_contract(&authorization_contract_path)?;
    uploader.send_single_contract(&processor_contract_path)?;

    uploader
        .with_chain_name(JUNO_CHAIN_NAME)
        .send_single_contract(&processor_contract_path)?;

    let processor_contract_code_id_on_neutron = test_ctx
        .get_contract()
        .contract("valence_processor")
        .get_cw()
        .code_id
        .unwrap();

    // We need to predict the authorization contract address in advance for the processor contract on the main domain
    let salt = hex::encode("authorization");
    let predicted_authorization_contract_address = test_ctx
        .get_built_contract_address()
        .src(NEUTRON_CHAIN_NAME)
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .contract("valence_authorization")
        .salt_hex_encoded(&salt)
        .get();

    // Now we can instantiate the processor
    let processor_instantiate_msg = valence_processor_utils::msg::InstantiateMsg {
        authorization_contract: predicted_authorization_contract_address,
        polytone_contracts: None,
    };

    let processor_on_main_domain = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        processor_contract_code_id_on_neutron,
        &serde_json::to_string(&processor_instantiate_msg).unwrap(),
        "processor",
        None,
        "",
    )
    .unwrap();

    info!(
        "Contract of processor on main domain: {}",
        processor_on_main_domain.address
    );

    Ok(())
}
