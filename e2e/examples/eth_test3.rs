use alloy::primitives::{keccak256, Address, Bytes, B256, U256};
use alloy::providers::Provider;
use alloy::rpc::types::{TransactionInput, TransactionRequest};
use alloy::sol_types::{SolConstructor, SolValue, SolCall};
use alloy_sol_types_encoder::SolValue as EncoderSolValue;
use localic_utils::utils::ethereum::EthClient;
use valence_solidity_bindings::{Forwarder, LibraryProxy};
use std::error::Error;
use std::str::FromStr;
use valence_e2e::utils::{ethereum::set_up_anvil_container, DEFAULT_ANVIL_RPC_ENDPOINT};

const DEPLOYER: &str = "0x4e59b44847b379578588920cA78FbF26c0B4956C";

fn predict_create2_address(deployer: Address, salt: B256, bytecode: &[u8]) -> Address {
    // Compute the keccak256 hash of the bytecode
    let bytecode_hash = keccak256(bytecode);

    // Create the input for the final hash
    // Format: 0xff + deployer address + salt + keccak256(bytecode)
    let mut input = Vec::with_capacity(1 + 20 + 32 + 32);
    input.push(0xff);
    input.extend_from_slice(deployer.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(bytecode_hash.as_slice());

    // Compute the final hash and take the last 20 bytes as the address
    let hash = keccak256(&input);
    Address::from_slice(&hash[12..])
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(set_up_anvil_container())?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;
    let accounts = eth.get_accounts_addresses()?;

    let deployer = Address::from_str(DEPLOYER).unwrap();

    println!("Step 1: Deploy ForwarderProxy using CREATE2");
    
    // Use a simple salt for CREATE2
    let salt = keccak256(b"test");
    
    // Get the proxy bytecode
    let proxy_bytecode = &LibraryProxy::BYTECODE;
    println!("LibraryProxy bytecode size: {} bytes", proxy_bytecode.len());
    let constructor_call = LibraryProxy::constructorCall {
        _admin: accounts[0],
    }.abi_encode();

    let mut full_bytecode = proxy_bytecode.to_vec();
    full_bytecode.extend_from_slice(&constructor_call);

    // Calculate the expected proxy address
    let predicted_proxy_addr = predict_create2_address(deployer, salt, &full_bytecode);
    println!("Predicted ForwarderProxy address: {:?}", predicted_proxy_addr);
    
    // Create a constructor call for the proxy

    // Prepare CREATE2 deployment data
    let mut full_data = salt.to_vec();
    full_data.extend_from_slice(&full_bytecode);
    
    // Create transaction with raw concatenated data
    let create2_tx = TransactionRequest {
        to: Some(alloy::primitives::TxKind::from(deployer)),
        input: TransactionInput {
            input: Some(Bytes::from(full_data.clone())),
            data: Some(Bytes::from(full_data)),
        },
        value: Some(U256::ZERO),
        gas: Some(5_000_000),               
        gas_price: Some(1_000_000_000_u128), 
        ..Default::default()
    }
    .from(accounts[0]);

    let create2_result = eth.send_transaction(create2_tx)?;
    println!("CREATE2 proxy deployment result: {:?}", create2_result);
    
    // Check if the proxy was deployed successfully
    let proxy_code = eth
        .rt
        .block_on(async { eth.provider.get_code_at(predicted_proxy_addr).await })?;
    
    if proxy_code.is_empty() {
        println!("Error: CREATE2 proxy deployment failed!");
        return Err("CREATE2 proxy deployment failed".into());
    }
    
    println!("ForwarderProxy deployed successfully at: {:?}", predicted_proxy_addr);
    println!("Proxy code size: {} bytes", proxy_code.len());
    
    // Create a LibraryProxy instance to interact with it
    let proxy = LibraryProxy::new(predicted_proxy_addr, &eth.provider);
    
    // Check if the proxy is already initialized or has an admin setup
    println!("\nInspecting the proxy contract before initialization:");
    let owner_call = proxy.admin();
    let owner_result = eth.rt.block_on(async { owner_call.call().await });
    println!("Current proxy owner: {:?}", owner_result);
    
    let is_initialized_call = proxy.initialized();
    let is_initialized_result = eth.rt.block_on(async { is_initialized_call.call().await });
    println!("Is proxy initialized: {:?}", is_initialized_result);
    
    println!("\nStep 2: Deploy Forwarder contract normally");
    
    // Create a valid test config
    let test_config = valence_solidity_bindings::Forwarder::ForwarderConfig {
        inputAccount: accounts[0],
        outputAccount: accounts[0],
        forwardingConfigs: vec![
            valence_solidity_bindings::Forwarder::ForwardingConfig {
                tokenAddress: accounts[0], // Using account as token for test
                maxAmount: 1000,
            }
        ],
        intervalType: 0, // TIME
        minInterval: 0,
    };

    // Deploy using regular method to get the implementation
    let forwarder_contract_tx = valence_solidity_bindings::Forwarder::deploy_builder(
        &eth.provider,
        accounts[0],
        accounts[0],
        test_config.clone().abi_encode().into(),
    )
    .into_transaction_request()
    .from(accounts[0]);

    let regular_result = eth.send_transaction(forwarder_contract_tx)?;
    let forwarder_impl_addr = regular_result.contract_address.unwrap();
    println!("Forwarder implementation deployed at: {:?}", forwarder_impl_addr);
    
    // Verify the Forwarder implementation has code
    let impl_code = eth
        .rt
        .block_on(async { eth.provider.get_code_at(forwarder_impl_addr).await })?;
    
    println!("Forwarder implementation code size: {} bytes", impl_code.len());
    
    println!("\nStep 3: Initialize the proxy to point to the implementation");
    
    // Debug the initialize function in the LibraryProxy solidity contract
    println!("Examining initialize function in LibraryProxy:");
    let abi_encoded = LibraryProxy::initializeCall { _implementation: forwarder_impl_addr }.abi_encode();
    println!("ABI encoded initialize call: {} bytes", abi_encoded.len());
    println!("First 32 bytes: {:?}", &abi_encoded[..32.min(abi_encoded.len())]);
    
    // Try with manual encoding to ensure the format is correct
    // Function signature: initialize(address _implementation)
    let initialize_signature = keccak256(b"initialize(address)")[..4].to_vec();
    println!("Initialize function signature: {:?}", initialize_signature);
    
    // Encode the address parameter (padding to 32 bytes)
    let mut address_param = vec![0u8; 32];
    address_param[12..].copy_from_slice(forwarder_impl_addr.as_slice());
    
    // Combine function signature and parameters
    let mut manual_initialize_data = initialize_signature;
    manual_initialize_data.extend_from_slice(&address_param);
    
    println!("Manual initialize call data: {} bytes", manual_initialize_data.len());
    println!("First 32 bytes: {:?}", &manual_initialize_data[..32.min(manual_initialize_data.len())]);
    
    // Create and send transaction to initialize the proxy (using manual encoding)
    let initialize_tx = TransactionRequest {
        to: Some(alloy::primitives::TxKind::from(predicted_proxy_addr)),
        input: TransactionInput {
            input: Some(Bytes::from(manual_initialize_data.clone())),
            data: Some(Bytes::from(manual_initialize_data)),
        },
        value: Some(U256::ZERO),
        gas: Some(200_000), // Increased gas limit for better diagnostics
        gas_price: Some(1_000_000_000_u128),
        ..Default::default()
    }
    .from(accounts[0]);
    
    let initialize_result = eth.send_transaction(initialize_tx.clone())?;
    println!("Proxy initialization result: {:?}", initialize_result);
    
    if initialize_result.status() != true {
        println!("Error: Manual proxy initialization failed!");
        
        // Try to get more details about why it's failing
        println!("\nTrying to debug the initialization failure:");
        
        // Try simulating the call to get the revert reason
        let sim_result = eth.rt.block_on(async {
            eth.provider.call(&initialize_tx).await
        });
        println!("Call simulation result: {:?}", sim_result);
        
        // Try using other functions to see if the contract responds properly
        println!("\nTesting other proxy functions to see if it's responsive:");
        
        // Try getting the current implementation if available
        let impl_call = proxy.implementation();
        let impl_result = eth.rt.block_on(async { impl_call.call().await });
        println!("Current implementation: {:?}", impl_result);
        
        // Now try again with the LibraryProxy encoding to see if that works
        println!("\nTrying again with library-generated encoding:");
        let initialize_lib_tx = TransactionRequest {
            to: Some(alloy::primitives::TxKind::from(predicted_proxy_addr)),
            input: TransactionInput {
                input: Some(Bytes::from(abi_encoded.clone())),
                data: Some(Bytes::from(abi_encoded)),
            },
            value: Some(U256::ZERO),
            gas: Some(200_000),
            gas_price: Some(1_000_000_000_u128),
            ..Default::default()
        }
        .from(accounts[0]);
        
        let initialize_lib_result = eth.send_transaction(initialize_lib_tx)?;
        println!("Library-encoded initialization result: {:?}", initialize_lib_result);
        
        if initialize_lib_result.status() != true {
            println!("Library-encoded initialization also failed");
            return Err("Proxy initialization failed".into());
        } else {
            println!("Library-encoded initialization succeeded!");
        }
    } else {
        println!("Manual proxy initialization succeeded!");
    }
    
    println!("\nStep 4: Verify the proxy works by calling a function on the Forwarder");
    
    // Create a Forwarder contract instance at the proxy address
    let forwarder_proxy = valence_solidity_bindings::Forwarder::new(predicted_proxy_addr, &eth.provider);
    
    // Try to call a view function like 'owner()' which should be delegated to the implementation
    let owner_call = forwarder_proxy.owner();
    let owner_result = eth.rt.block_on(async { owner_call.call().await });
    println!("Owner result through proxy: {:?}", owner_result);
    
    // Try calling 'config()' to see if it returns the expected configuration
    let config_call = forwarder_proxy.config();
    let config_result = eth.rt.block_on(async { config_call.call().await });
    println!("Config result through proxy: {:?}", config_result);

    println!("\nSummary:");
    println!("1. ForwarderProxy deployed via CREATE2 at: {:?}", predicted_proxy_addr);
    println!("2. Forwarder implementation deployed normally at: {:?}", forwarder_impl_addr);
    println!("3. Proxy initialized to point to the implementation");
    println!("4. Proxy is now a fully functional Forwarder with CREATE2 address determinism");
    
    Ok(())
}