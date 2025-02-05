use std::{collections::HashMap, fs, path::Path};

use alloy::{hex::FromHex, primitives::FixedBytes};
use localic_std::modules::cosmwasm::{contract_execute, contract_instantiate};
use localic_utils::{
    utils::{ethereum::EthClient, test_context::TestContext},
    DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
    NEUTRON_CHAIN_PREFIX,
};
use log::{error, info};
use serde_json::{json, Value};

use super::{
    solidity_contracts::{
        InterchainGasPaymaster, Mailbox, MerkleTreeHook, PausableIsm, ValidatorAnnounce,
    },
    GAS_FLAGS, HYPERLANE_COSMWASM_ARTIFACTS_PATH, HYPERLANE_RELAYER_CONFIG_PATH,
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON, NEUTRON_HYPERLANE_DOMAIN,
};
use bollard::{
    container::{Config, CreateContainerOptions},
    secret::{HostConfig, Mount, MountTypeEnum},
    Docker,
};
use bollard::{
    container::{ListContainersOptions, RemoveContainerOptions, StopContainerOptions},
    network::{ConnectNetworkOptions, CreateNetworkOptions, ListNetworksOptions},
    secret::EndpointSettings,
};

pub struct HyperlaneContracts {
    pub mailbox: String,
    pub hook_merkle: String,
    pub igp: String,
    pub ism_pausable: String,
    pub validator_announce: String,
}

/// Converts a bech32 address to a hex address equivalent
pub fn bech32_to_hex_address(bech32_address: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Decode the bech32 address
    let (_, data) = bech32::decode(bech32_address)?;
    // Convert to hex and add 0x prefix
    let hex = format!("0x{}", hex::encode(data));
    Ok(hex)
}

/// Converts a bech32 address into a bytes32 equivalent with padded zeroes
pub fn bech32_to_evm_bytes32(
    bech32_address: &str,
) -> Result<FixedBytes<32>, Box<dyn std::error::Error>> {
    // Decode the bech32 address
    let (_, data) = bech32::decode(bech32_address)?;
    // Convert to hex
    let address_hex = hex::encode(data);
    // Pad with zeroes to 32 bytes
    let padded_hex = format!("{:0>64}", address_hex);
    // Convert to FixedBytes
    let address_in_bytes32 = FixedBytes::<32>::from_hex(padded_hex)?;

    Ok(address_in_bytes32)
}

// Function to set up CosmWasm Hyperlane contracts
// Creates and initializes all required contracts for Hyperlane functionality on a CosmWasm chain
pub fn set_up_cw_hyperlane_contracts(
    test_ctx: &mut TestContext,
) -> Result<HyperlaneContracts, Box<dyn std::error::Error>> {
    // Initialize contract uploader with test context
    let mut uploader = test_ctx.build_tx_upload_contracts();
    // Upload contracts using local cache for optimization
    uploader
        .send_with_local_cache(
            HYPERLANE_COSMWASM_ARTIFACTS_PATH,
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )
        .unwrap();

    // Get the code ID for the mailbox contract from the test context
    let mailbox_code_id = test_ctx
        .get_contract()
        .contract("hpl_mailbox")
        .get_cw()
        .code_id
        .unwrap();

    // Create instantiation message for mailbox contract with chain-specific parameters
    let mailbox_instantiate_msg = hpl_interface::core::mailbox::InstantiateMsg {
        hrp: NEUTRON_CHAIN_PREFIX.to_string(),
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        domain: NEUTRON_HYPERLANE_DOMAIN,
    };

    // Instantiate the mailbox contract
    let mailbox = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        mailbox_code_id,
        &serde_json::to_string(&mailbox_instantiate_msg).unwrap(),
        "mailbox",
        None,
        "",
    )
    .unwrap()
    .address;

    // Get code ID for merkle hook contract
    let merkle_hook_code_id = test_ctx
        .get_contract()
        .contract("hpl_hook_merkle")
        .get_cw()
        .code_id
        .unwrap();

    // Create instantiation message for merkle hook contract
    let hook_merkle_instantiate_msg = hpl_interface::hook::merkle::InstantiateMsg {
        mailbox: mailbox.clone(),
    };

    // Instantiate merkle hook contract
    let hook_merkle = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        merkle_hook_code_id,
        &serde_json::to_string(&hook_merkle_instantiate_msg).unwrap(),
        "hook_merkle",
        None,
        "",
    )
    .unwrap()
    .address;

    // Get code ID for IGP (Interchain Gas Paymaster) contract
    let igp_code_id = test_ctx
        .get_contract()
        .contract("hpl_igp")
        .get_cw()
        .code_id
        .unwrap();

    // Create instantiation message for IGP contract with chain-specific parameters
    let igp_instantiate_msg = hpl_interface::igp::core::InstantiateMsg {
        hrp: NEUTRON_CHAIN_PREFIX.to_string(),
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        gas_token: NEUTRON_CHAIN_DENOM.to_string(),
        beneficiary: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        default_gas_usage: 0,
    };

    // Instantiate IGP contract
    // Note: Using serde_json_wasm for compatibility with older versions that serialize u128 as string
    let igp = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        igp_code_id,
        &serde_json_wasm::to_string(&igp_instantiate_msg).unwrap(),
        "igp",
        None,
        "",
    )
    .unwrap()
    .address;

    // Get code ID for pausable ISM (Interchain Security Module) contract
    let ism_pausable_code_id = test_ctx
        .get_contract()
        .contract("hpl_ism_pausable")
        .get_cw()
        .code_id
        .unwrap();

    // Create instantiation message for pausable ISM contract
    let ism_pausable_instantiate_msg = hpl_interface::ism::pausable::InstantiateMsg {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        paused: false,
    };

    // Instantiate pausable ISM contract
    let ism_pausable = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        ism_pausable_code_id,
        &serde_json::to_string(&ism_pausable_instantiate_msg).unwrap(),
        "ism_pausable",
        None,
        "",
    )
    .unwrap()
    .address;

    // Get code ID for validator announce contract
    let validator_announce_code_id = test_ctx
        .get_contract()
        .contract("hpl_validator_announce")
        .get_cw()
        .code_id
        .unwrap();

    // Create instantiation message for validator announce contract
    let validator_announce_instantiate_msg = hpl_interface::core::va::InstantiateMsg {
        hrp: NEUTRON_CHAIN_PREFIX.to_string(),
        mailbox: mailbox.clone(),
    };

    // Instantiate validator announce contract
    let validator_announce = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        validator_announce_code_id,
        &serde_json::to_string(&validator_announce_instantiate_msg).unwrap(),
        "validator_announce",
        None,
        "",
    )
    .unwrap()
    .address;

    // Configure mailbox contract with hooks and ISM
    let mailbox_set_default_hook_msg = hpl_interface::core::mailbox::ExecuteMsg::SetDefaultHook {
        hook: hook_merkle.clone(),
    };
    let mailbox_set_required_hook_msg = hpl_interface::core::mailbox::ExecuteMsg::SetRequiredHook {
        hook: hook_merkle.clone(),
    };
    let mailbox_set_default_ism_msg = hpl_interface::core::mailbox::ExecuteMsg::SetDefaultIsm {
        ism: ism_pausable.clone(),
    };

    // Execute messages to set up mailbox configuration with delays between operations
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&mailbox_set_default_hook_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&mailbox_set_required_hook_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &mailbox,
        DEFAULT_KEY,
        &serde_json::to_string(&mailbox_set_default_ism_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Return struct containing all deployed contract addresses
    Ok(HyperlaneContracts {
        mailbox,
        hook_merkle,
        igp,
        ism_pausable,
        validator_announce,
    })
}

// Function to set up Ethereum Hyperlane contracts
// Deploys and initializes all required contracts for Hyperlane functionality on an Ethereum chain
pub fn set_up_eth_hyperlane_contracts(
    eth_client: &EthClient,
    domain_id: u32,
) -> Result<HyperlaneContracts, Box<dyn std::error::Error>> {
    // Get list of available Ethereum accounts
    let accounts = eth_client.get_accounts_addresses()?;

    // Deploy mailbox contract with specified domain ID
    let transaction = Mailbox::deploy_builder(&eth_client.provider, domain_id)
        .into_transaction_request()
        .from(accounts[0]);
    let mailbox = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    // Deploy merkle tree hook contract linked to mailbox
    let transaction = MerkleTreeHook::deploy_builder(&eth_client.provider, mailbox)
        .into_transaction_request()
        .from(accounts[0]);
    let hook_merkle = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    // Deploy interchain gas paymaster contract
    let transaction = InterchainGasPaymaster::deploy_builder(&eth_client.provider)
        .into_transaction_request()
        .from(accounts[0]);
    let igp = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    // Deploy pausable ISM contract
    let transaction = PausableIsm::deploy_builder(&eth_client.provider, accounts[0])
        .into_transaction_request()
        .from(accounts[0]);
    let ism_pausable = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    // Deploy validator announce contract
    let transaction = ValidatorAnnounce::deploy_builder(&eth_client.provider, mailbox)
        .into_transaction_request()
        .from(accounts[0]);
    let validator_announce = eth_client
        .send_transaction(transaction)?
        .contract_address
        .unwrap();

    // Initialize mailbox with hooks and ISM
    let mailbox_contract = Mailbox::new(mailbox, &eth_client.provider);
    let tx = mailbox_contract
        .initialize(accounts[0], ism_pausable, hook_merkle, hook_merkle)
        .into_transaction_request()
        .from(accounts[0]);
    eth_client.send_transaction(tx)?;

    // Return struct containing all deployed contract addresses
    Ok(HyperlaneContracts {
        mailbox: mailbox.to_string(),
        hook_merkle: hook_merkle.to_string(),
        igp: igp.to_string(),
        ism_pausable: ism_pausable.to_string(),
        validator_announce: validator_announce.to_string(),
    })
}

/// Set up the complete Hyperlane environment
/// This includes creating Docker network, connecting containers, and starting the relayer
pub fn set_up_hyperlane(
    docker_network_name: &str,
    docker_container_names: Vec<&str>,
    chain1: &str,
    chain2: &str,
    contracts_chain1: &HyperlaneContracts,
    contracts_chain2: &HyperlaneContracts,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Docker connection and runtime
    let docker = Docker::connect_with_local_defaults()?;
    let rt = tokio::runtime::Runtime::new()?;

    // Stop any existing relayer container to avoid conflicts
    rt.block_on(stop_existing_relayer(&docker))?;

    // Create Docker network if it doesn't exist and add containers
    match rt.block_on(create_docker_network(&docker, docker_network_name))? {
        true => {
            rt.block_on(add_containers_to_network(
                &docker,
                docker_container_names,
                docker_network_name,
            ))?;
        }
        false => info!("Network already exists"),
    }

    // Update Hyperlane configuration with new contract addresses
    update_hyperlane_config(chain1, chain2, contracts_chain1, contracts_chain2)?;

    // Start the Hyperlane relayer
    rt.block_on(run_hyperlane_relayer(
        &docker,
        docker_network_name,
        chain1,
        chain2,
    ))?;

    Ok(())
}

/// Creates a Docker network if it doesn't already exist
/// Returns true if network was created, false if it already existed
async fn create_docker_network(
    docker: &Docker,
    network_name: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Check if network already exists
    if !docker
        .list_networks(Some(ListNetworksOptions::<String> {
            filters: Default::default(),
        }))
        .await?
        .iter()
        .any(|n| n.name == Some(network_name.to_string()))
    {
        // Create network with default options if it doesn't exist
        let network_options = CreateNetworkOptions {
            name: network_name,
            ..Default::default()
        };

        match docker.create_network(network_options).await {
            Ok(_) => info!("Network created successfully"),
            Err(e) => error!("Failed to create network: {}", e),
        }

        return Ok(true);
    }

    Ok(false)
}

/// Add specified containers to the Docker network
async fn add_containers_to_network(
    docker: &Docker,
    container_names: Vec<&str>,
    network_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Set up filter to find containers by name
    let mut filters = HashMap::new();
    filters.insert("name", container_names);

    let list_container_options = ListContainersOptions {
        filters,
        all: true,
        ..Default::default()
    };

    // Get list of matching containers
    let containers = docker.list_containers(Some(list_container_options)).await?;

    // Connect each container to the network
    for container in containers {
        if let Some(id) = container.id {
            let connect_options = ConnectNetworkOptions {
                container: id.as_str(),
                endpoint_config: EndpointSettings::default(),
            };

            match docker.connect_network(network_name, connect_options).await {
                Ok(_) => info!("Connected container {} to network", id),
                Err(e) => error!("Failed to connect container {}: {}", id, e),
            }
        }
    }

    Ok(())
}

/// Stop and remove any existing Hyperlane relayer container
async fn stop_existing_relayer(docker: &Docker) -> Result<(), Box<dyn std::error::Error>> {
    // Define relayer container name
    let relayer_name = "hyperlane-relayer";

    // Set up filter to find relayer container
    let mut filters = HashMap::new();
    filters.insert("name", vec![relayer_name]);

    let options = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };

    // Find existing relayer containers
    let containers = docker.list_containers(Some(options)).await?;

    // Stop and remove each found container
    for container in containers {
        if let Some(id) = container.id {
            // Attempt to stop container first (ignore errors if already stopped)
            let _ = docker
                .stop_container(&id, None::<StopContainerOptions>)
                .await;

            // Remove the container with force option
            match docker
                .remove_container(
                    &id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await
            {
                Ok(_) => info!("Removed existing relayer container: {}", id),
                Err(e) => error!("Failed to remove container {}: {}", id, e),
            }
        }
    }

    Ok(())
}

/// Run the Hyperlane relayer in a Docker container
/// Sets up necessary directories and mounts for the relayer
async fn run_hyperlane_relayer(
    docker: &Docker,
    network_name: &str,
    chain1: &str,
    chain2: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = std::env::current_dir()?.join("local-interchaintest");

    let paths = ["hyperlane_db_relayer", "tmp/hyperlane_validator_signatures"];

    // Clean up and recreate directories
    for path in &paths {
        let full_path = base_dir.join(path);
        if full_path.exists() {
            fs::remove_dir_all(&full_path)?;
        }
        std::fs::create_dir_all(&full_path)?;
    }

    let config_path = base_dir.join("hyperlane/config/config.json");
    let config_path_str = config_path.to_str().unwrap();

    // Define mount configurations
    let mut mounts = vec![Mount {
        target: Some(config_path_str.to_string()),
        source: Some(config_path_str.to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(true),
        ..Default::default()
    }];

    // Add DB mount
    mounts.push(Mount {
        target: Some("/hyperlane_db".to_string()),
        source: Some(base_dir.join(paths[0]).to_str().unwrap().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(false),
        ..Default::default()
    });

    // Add validator signatures mount
    mounts.push(Mount {
        target: Some("/tmp/hyperlane_validator_signatures".to_string()),
        source: Some(base_dir.join(paths[1]).to_str().unwrap().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(true),
        ..Default::default()
    });

    let config_files = format!("CONFIG_FILES={}", config_path_str);
    let relay_chains = format!("{},{}", chain1, chain2);

    let config = Config {
        image: Some("gcr.io/abacus-labs-dev/hyperlane-agent:agents-v1.0.0"),
        cmd: Some(vec![
            "./relayer",
            "--db",
            "/hyperlane_db",
            "--relayChains",
            &relay_chains,
            "--defaultSigner.key",
            "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
        ]),
        env: Some(vec![&config_files]),
        host_config: Some(HostConfig {
            network_mode: Some(network_name.to_string()),
            mounts: Some(mounts),
            ..Default::default()
        }),
        tty: Some(true),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: "hyperlane-relayer",
        platform: None,
    };

    let container = docker.create_container(Some(options), config).await?;
    docker
        .start_container::<String>(&container.id, None)
        .await?;
    info!("Started relayer container: {}", container.id);

    Ok(())
}

/// Update the contract addresses in the configuration for a specific chain
pub fn update_chain_contracts(
    config: &mut Value,
    chain_name: &str,
    contracts: &HyperlaneContracts,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get chain configuration object
    let chain = config["chains"][chain_name]
        .as_object_mut()
        .ok_or("Chain not found in config")?;

    // Helper function to convert bech32 addresses to hex if needed
    let process_address = |addr: &str| -> Result<String, Box<dyn std::error::Error>> {
        if addr.starts_with("0x") {
            Ok(addr.to_string())
        } else {
            bech32_to_hex_address(addr)
        }
    };

    // Update contract addresses in configuration
    let mailbox_addr = process_address(&contracts.mailbox)?;
    chain["mailbox"] = json!(mailbox_addr);

    let hook_addr = process_address(&contracts.hook_merkle)?;
    chain["merkleTreeHook"] = json!(hook_addr);

    let igp_addr = process_address(&contracts.igp)?;
    chain["interchainGasPaymaster"] = json!(igp_addr);

    let validator_addr = process_address(&contracts.validator_announce)?;
    chain["validatorAnnounce"] = json!(validator_addr);

    Ok(())
}

/// Update the Hyperlane configuration file with new contract addresses for both chains
pub fn update_hyperlane_config(
    chain1_name: &str,
    chain2_name: &str,
    contracts_chain1: &HyperlaneContracts,
    contracts_chain2: &HyperlaneContracts,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Read existing configuration file
    let env = std::env::current_dir()?;
    let config_path = env.join(Path::new(HYPERLANE_RELAYER_CONFIG_PATH));
    let config_str = fs::read_to_string(config_path.clone())?;
    let mut config: Value = serde_json::from_str(&config_str)?;

    // Update configuration for both chains
    update_chain_contracts(&mut config, chain1_name, contracts_chain1)?;
    update_chain_contracts(&mut config, chain2_name, contracts_chain2)?;

    // Write updated configuration back to file
    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;

    Ok(config)
}
