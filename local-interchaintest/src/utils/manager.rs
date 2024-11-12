use std::{collections::HashMap, env, error::Error, fs};

use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use valence_program_manager::{
    config::{ChainInfo, GLOBAL_CONFIG},
    error::ManagerResult,
    init_program,
    program_config::ProgramConfig,
    program_update::{ProgramConfigUpdate, UpdateResponse},
    update_program,
};

const LOG_FILE_PATH: &str = "local-interchaintest/configs/logs.json";

pub const REGISTRY_NAME: &str = "valence_program_registry";
pub const AUTHORIZATION_NAME: &str = "valence_authorization";
pub const PROCESSOR_NAME: &str = "valence_processor";
pub const BASE_ACCOUNT_NAME: &str = "valence_base_account";
pub const SPLITTER_NAME: &str = "valence_splitter_library";
pub const REVERSE_SPLITTER_NAME: &str = "valence_reverse_splitter_library";
pub const FORWARDER_NAME: &str = "valence_forwarder_library";
pub const GENERIC_IBC_TRANSFER_NAME: &str = "valence-generic-ibc-transfer-library";
pub const NEUTRON_IBC_TRANSFER_NAME: &str = "valence-neutron-ibc-transfer-library";
pub const ASTROPORT_LPER_NAME: &str = "valence_astroport_lper";
pub const ASTROPORT_WITHDRAWER_NAME: &str = "valence_astroport_withdrawer";

/// Those contracts will always be uploaded because each program needs them
const BASIC_CONTRACTS: [&str; 2] = [PROCESSOR_NAME, BASE_ACCOUNT_NAME];

/// Setup everything that is needed for the manager to run, including uploading the libraries
///
/// You can pass a list of contracts to upload, authorization, processor and base account are always uploaded,
/// you need to specify the contracts you want to be uploaded for the given test
pub fn setup_manager(
    test_ctx: &mut TestContext,
    chains_file_path: &str,
    exclude_chains: Vec<&str>,
    mut contracts: Vec<&str>,
) -> Result<(), Box<dyn Error>> {
    let curr_path = env::current_dir()?;
    let artifacts_dir = format!("{}/artifacts", curr_path.to_str().unwrap());
    let chain_infos = get_chain_infos(chains_file_path);
    let mut gc = get_global_config();
    gc.chains.clone_from(&chain_infos);

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.with_chain_name(NEUTRON_CHAIN_NAME);

    // combine the basic contracts with the contracts passed
    BASIC_CONTRACTS.iter().for_each(|s| {
        if !contracts.contains(s) {
            contracts.push(s);
        }
    });

    let authorization_contract_path = format!("{artifacts_dir}/{AUTHORIZATION_NAME}.wasm");
    let registry_contract_path = format!("{artifacts_dir}/{REGISTRY_NAME}.wasm");

    // Authorization and registry are special because they should only be uploaded to neutron
    uploader.send_single_contract(&authorization_contract_path)?;
    uploader.send_single_contract(&registry_contract_path)?;

    let authorization_code_id = test_ctx
        .get_contract()
        .contract(AUTHORIZATION_NAME)
        .get_cw()
        .code_id
        .unwrap();
    let registry_code_id = test_ctx
        .get_contract()
        .contract(REGISTRY_NAME)
        .get_cw()
        .code_id
        .unwrap();

    // Upload all contracts
    for (chain_name, _) in chain_infos.iter() {
        if exclude_chains.contains(&chain_name.as_str()) {
            continue;
        }
        let mut code_ids_map = HashMap::new();
        // if chain is neutron, we add authorization code id
        if chain_name == NEUTRON_CHAIN_NAME {
            code_ids_map.insert(AUTHORIZATION_NAME.to_string(), authorization_code_id);
        }

        for contract_name in contracts.iter() {
            let mut uploader = test_ctx.build_tx_upload_contracts();
            uploader.with_chain_name(chain_name);
            let (path, contrat_wasm_name, contract_name) =
                get_contract_path(chain_name, contract_name, artifacts_dir.as_str());

            // Upload contract
            uploader.send_single_contract(path.as_str()).unwrap();

            // get its code id
            let code_id = test_ctx
                .get_contract()
                .contract(contrat_wasm_name)
                .get_cw()
                .code_id
                .unwrap();

            code_ids_map.insert(contract_name.to_string(), code_id);
        }

        gc.contracts
            .code_ids
            .insert(chain_name.clone(), code_ids_map);
    }

    // init the registry on neutron
    let registry = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        registry_code_id,
        &serde_json::to_string(&valence_program_registry_utils::InstantiateMsg {
            admin: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        })
        .unwrap(),
        "program-registry",
        None,
        "",
    )
    .unwrap();

    gc.general.registry_addr = registry.address;

    Ok(())
}

/// A way to get specific contract path based on what chains we upload contract on.
/// Need to add paths manually here
/// The return is (path, contract wasm file name, contract name)
fn get_contract_path<'a>(
    chain_name: &str,
    contract_name: &'a str,
    artifacts_dir: &'a str,
) -> (String, &'a str, &'a str) {
    if NEUTRON_IBC_TRANSFER_NAME.contains(contract_name)
        || GENERIC_IBC_TRANSFER_NAME.contains(contract_name)
        || contract_name == "ibc-transfer"
    {
        if chain_name == NEUTRON_CHAIN_NAME {
            return (
                format!("{}/{}.wasm", artifacts_dir, NEUTRON_IBC_TRANSFER_NAME),
                NEUTRON_IBC_TRANSFER_NAME,
                "ibc_transfer",
            );
        } else {
            return (
                format!("{}/{}.wasm", artifacts_dir, GENERIC_IBC_TRANSFER_NAME),
                GENERIC_IBC_TRANSFER_NAME,
                "ibc_transfer",
            );
        }
    }

    (
        format!("{}/{}.wasm", artifacts_dir, contract_name),
        contract_name,
        contract_name,
    )
}

/// Get the chain infos and bridge info from the log file
fn get_chain_infos(chains_file_path: &str) -> HashMap<String, ChainInfo> {
    let log_file = fs::File::open(LOG_FILE_PATH).expect("file should open read only");
    let log_json: serde_json::Value =
        serde_json::from_reader(log_file).expect("file should be proper JSON");
    let log_json = log_json
        .get("chains")
        .expect("file should have chains")
        .as_array()
        .expect("Chains should be an array");

    let curr_path = env::current_dir().unwrap();
    let chain_file_path = format!(
        "{}/local-interchaintest/chains/{}",
        curr_path.to_str().unwrap(),
        chains_file_path
    );
    let chain_file = fs::File::open(chain_file_path).expect("file should open read only");
    let chain_json: serde_json::Value =
        serde_json::from_reader(chain_file).expect("file should be proper JSON");
    let chain_json = chain_json
        .get("chains")
        .expect("file should have chains")
        .as_array()
        .expect("Chains should be an array");

    let mut chain_infos = HashMap::new();

    chain_json.iter().for_each(|chain_data| {
        let chain_name = chain_data
            .get("name")
            .expect("Chain data must have name")
            .as_str()
            .expect("name must be a string");
        let chain_id = chain_data
            .get("chain_id")
            .expect("Chain data must have chain_id")
            .as_str()
            .expect("chain_id must be a string");

        let log_chain_data = log_json
            .iter()
            .find(|log_chain| {
                log_chain
                    .get("chain_id")
                    .expect("Chain data must have chain_id")
                    .as_str()
                    .expect("chain_id must be a string")
                    .contains(chain_id)
            })
            .expect("Chain data must be in log file");

        let rpc = log_chain_data
            .get("rpc_address")
            .expect("Log chain data must have rpc_address")
            .as_str()
            .expect("rpc_address must be a string");
        let grpc = log_chain_data
            .get("grpc_address")
            .expect("Log chain data must have grpc_address")
            .as_str()
            .expect("grpc_address must be a string");
        let prefix = chain_data
            .get("bech32_prefix")
            .expect("Chain data must have bech32_prefix")
            .as_str()
            .expect("bech32_prefix must be a string");
        let gas_price = chain_data
            .get("gas_prices")
            .expect("Chain data must have gas_prices")
            .as_str()
            .expect("gas_prices must be a string");
        let gas_denom = chain_data
            .get("denom")
            .expect("Chain data must have denom")
            .as_str()
            .expect("denom must be a string");
        let coin_type = chain_data
            .get("coin_type")
            .expect("Chain data must have coin_type")
            .as_u64()
            .expect("coin_type must be a u64");

        let gas_price = parse_gas_price(gas_price);

        chain_infos.insert(
            chain_name.to_string(),
            ChainInfo {
                name: chain_name.to_string(),
                rpc: rpc.to_string(),
                grpc: format!("http://{}", grpc),
                prefix: prefix.to_string(),
                gas_price: gas_price.to_string(),
                gas_denom: gas_denom.to_string(),
                coin_type,
            },
        );
    });

    chain_infos
}

fn parse_gas_price(input: &str) -> String {
    // Split the input string by comma and take the first part
    let first_part = input.split(',').next().unwrap_or(input);

    // Find the position of the first non-digit character (excluding '.')
    let end_pos = first_part
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(first_part.len());

    // Extract the fractional number part as a string
    first_part[..end_pos].to_string()
}

/// Helper function to start manager init to hide the tokio block_on
pub fn use_manager_init(program_config: &mut ProgramConfig) -> ManagerResult<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(init_program(program_config))
}

/// Helper function to update manager config to hide the tokio block_on
pub fn use_manager_update(
    workflow_config_update: ProgramConfigUpdate,
) -> ManagerResult<UpdateResponse> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(update_program(workflow_config_update))
}

pub fn get_global_config(
) -> tokio::sync::MutexGuard<'static, valence_program_manager::config::Config> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(GLOBAL_CONFIG.lock())
}
