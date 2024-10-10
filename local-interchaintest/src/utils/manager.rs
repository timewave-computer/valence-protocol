use std::{collections::HashMap, env, error::Error, fs};

use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use valence_workflow_manager::{
    config::{ChainInfo, GLOBAL_CONFIG},
    error::ManagerResult,
    init_workflow,
    workflow_config::WorkflowConfig,
};

const LOG_FILE_PATH: &str = "local-interchaintest/configs/logs.json";

const AUTHORIZATION_NAME: &str = "valence_authorization";
const PROCESSOR_NAME: &str = "valence_processor";
const BASE_ACCOUNT_NAME: &str = "valence_base_account";
const SPLITTER_NAME: &str = "valence_splitter_service";
const REVERSE_SPLITTER_NAME: &str = "valence_reverse_splitter_service";
const FORWARDER_NAME: &str = "valence_forwarder_service";
const REGISTRY_NAME: &str = "valence_workflow_registry";

pub fn setup_manager(test_ctx: &mut TestContext) -> Result<(), Box<dyn Error>> {
    let artifacts_dir = format!("{}/artifacts", env::current_dir()?.to_str().unwrap());
    let chain_infos = get_data_from_log();
    let mut gc = get_global_config();
    gc.chains = chain_infos.clone();

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.with_chain_name(NEUTRON_CHAIN_NAME);

    // Get all contract paths
    let authorization_contract_path = format!("{artifacts_dir}/{AUTHORIZATION_NAME}.wasm");
    let processor_contract_path = format!("{artifacts_dir}/{PROCESSOR_NAME}.wasm");
    let base_account_contract_path = format!("{artifacts_dir}/{BASE_ACCOUNT_NAME}.wasm");
    let splitter_contract_path = format!("{artifacts_dir}/{SPLITTER_NAME}.wasm");
    let reverse_splitter_contract_path = format!("{artifacts_dir}/{REVERSE_SPLITTER_NAME}.wasm");
    let forwarder_contract_path = format!("{artifacts_dir}/{FORWARDER_NAME}.wasm");
    let registry_contract_path = format!("{artifacts_dir}/{REGISTRY_NAME}.wasm");

    // Upload all contracts
    uploader.send_single_contract(&authorization_contract_path)?;
    uploader.send_single_contract(&processor_contract_path)?;
    uploader.send_single_contract(&base_account_contract_path)?;
    uploader.send_single_contract(&splitter_contract_path)?;
    uploader.send_single_contract(&reverse_splitter_contract_path)?;
    uploader.send_single_contract(&forwarder_contract_path)?;
    uploader.send_single_contract(&registry_contract_path)?;

    // Get all contracts code ids
    let authorization_code_id = test_ctx
        .get_contract()
        .contract(AUTHORIZATION_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let processor_code_id = test_ctx
        .get_contract()
        .contract(PROCESSOR_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let base_account_code_id = test_ctx
        .get_contract()
        .contract(BASE_ACCOUNT_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let splitter_code_id = test_ctx
        .get_contract()
        .contract(SPLITTER_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let reverse_splitter_code_id = test_ctx
        .get_contract()
        .contract(REVERSE_SPLITTER_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let forwarder_code_id = test_ctx
        .get_contract()
        .contract(FORWARDER_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let registry_code_id = test_ctx
        .get_contract()
        .contract(REGISTRY_NAME)
        .get_cw()
        .code_id
        .unwrap();

    // Update config with all the code ids
    let mut code_ids_map = HashMap::new();
    code_ids_map.insert(AUTHORIZATION_NAME.to_string(), authorization_code_id);
    code_ids_map.insert(PROCESSOR_NAME.to_string(), processor_code_id);
    code_ids_map.insert(BASE_ACCOUNT_NAME.to_string(), base_account_code_id);
    code_ids_map.insert(SPLITTER_NAME.to_string(), splitter_code_id);
    code_ids_map.insert(REVERSE_SPLITTER_NAME.to_string(), reverse_splitter_code_id);
    code_ids_map.insert(FORWARDER_NAME.to_string(), forwarder_code_id);

    gc.contracts
        .code_ids
        .insert(NEUTRON_CHAIN_NAME.to_string(), code_ids_map);

    // init the registry
    let registry_init_msg = valence_workflow_registry_utils::InstantiateMsg {
        admin: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
    };

    let registry = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        registry_code_id,
        &serde_json::to_string(&registry_init_msg).unwrap(),
        "workflow-registry",
        None,
        "",
    )
    .unwrap();

    gc.general.registry_addr = registry.address;

    // let code_ids = GLOBAL_CONFIG.read().unwrap().contracts.code_ids.clone();
    // println!("{:?}", code_ids);

    // read the log file
    // setup the chain info (and later bridge info) for the global config
    // upload contracts to the chains and get their ids, and update the global config
    // upload and instantiate a registry (for the manager) and update the global config

    Ok(())
}

/// Get the chain infos and bridge info from the log file
pub fn get_data_from_log() -> HashMap<String, ChainInfo> {
    let log_file = fs::File::open(LOG_FILE_PATH)
        .expect("file should open read only");
    let log_json: serde_json::Value =
        serde_json::from_reader(log_file).expect("file should be proper JSON");

    let mut chain_infos = HashMap::new();

    log_json
        .get("chains")
        .expect("file should have chains")
        .as_array()
        .expect("Chains should be an array")
        .iter()
        // TODO: change to map later for all chains we have
        .for_each(|chain_data| {
            let chain_name = chain_data
                .get("chain_name")
                .expect("Chain data must have chain_name")
                .as_str()
                .expect("chain_name must be a string")
                .replace("local", "");

            // TODO: We only want neutron for now, change later hardcoded values
            if !chain_name.contains("neutron") {
                return;
            }

            let rpc = chain_data
                .get("rpc_address")
                .expect("Chain data must have rpc_address")
                .as_str()
                .expect("rpc_address must be a string");
            let grpc = chain_data
                .get("grpc_address")
                .expect("Chain data must have grpc_address")
                .as_str()
                .expect("grpc_address must be a string");

            chain_infos.insert(
                NEUTRON_CHAIN_NAME.to_string(),
                ChainInfo {
                    name: NEUTRON_CHAIN_NAME.to_string(),
                    rpc: rpc.to_string(),
                    grpc: format!("http://{}", grpc),
                    prefix: "neutron".to_string(),
                    gas_price: "0.025".to_string(),
                    gas_denom: "untrn".to_string(),
                    coin_type: 118,
                },
            );
        });

    chain_infos
}

/// Helper function to start manager init to hide the tokio block_on
pub fn use_manager_init(workflow_config: &mut WorkflowConfig) -> ManagerResult<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(init_workflow(workflow_config))
}

pub fn get_global_config(
) -> tokio::sync::MutexGuard<'static, valence_workflow_manager::config::Config> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(GLOBAL_CONFIG.lock())
}
