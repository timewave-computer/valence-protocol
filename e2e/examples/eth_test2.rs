use alloy::primitives::{keccak256, Address, Bytes, B256, U256};
use alloy::providers::Provider;
use alloy::rpc::types::{TransactionInput, TransactionRequest};
use alloy::sol;
use alloy::sol_types::{ SolCall, SolConstructor, SolValue};
use alloy_primitives_encoder::bytes;
use localic_utils::utils::ethereum::EthClient;
use std::error::Error;
use std::str::FromStr;
use valence_e2e::utils::{ethereum::set_up_anvil_container, DEFAULT_ANVIL_RPC_ENDPOINT};

const DEPLOYER: &str = "0x4e59b44847b379578588920cA78FbF26c0B4956C";

sol!(
    constructor(address _owner, address _processor);
);

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
    
    // Use a simple salt for CREATE2
    let salt = keccak256(b"test");
    
    // Get the proxy bytecode
    let forwarder_bytecode = &valence_solidity_bindings::Forwarder::BYTECODE;
    println!("forwarder_bytecode bytecode size: {} bytes", forwarder_bytecode.len());
    
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
    }.abi_encode();

    let constructor_call = valence_solidity_bindings::Forwarder::constructorCall {
        _owner: accounts[0],
        _processor: accounts[0],
        _config: test_config.clone().into(),
    }.abi_encode();

    let mut full_bytecode = forwarder_bytecode.to_vec();
    full_bytecode.extend_from_slice(&constructor_call);

    // Calculate the expected proxy address
    let predicted_forwarder_addr = predict_create2_address(deployer, salt, &full_bytecode);
    let predicted_forwarder_addr2 = deployer.create2_from_code(salt, &full_bytecode);
    println!("Predicted ForwarderProxy address: {:?}", predicted_forwarder_addr);
    println!("predicted_forwarder address: {:?} | {:?}", predicted_forwarder_addr, predicted_forwarder_addr2);
    
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
    let predicted_forwarder_code = eth
        .rt
        .block_on(async { eth.provider.get_code_at(predicted_forwarder_addr).await })?;
    
    if predicted_forwarder_code.is_empty() {
        println!("Error: CREATE2 proxy deployment failed!");
        return Err("CREATE2 proxy deployment failed".into());
    }
    
    println!("ForwarderProxy deployed successfully at: {:?}", predicted_forwarder_addr);
    println!("Proxy code size: {} bytes", predicted_forwarder_code.len());
    
    Ok(())
}