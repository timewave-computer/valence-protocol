use alloy::network::TransactionBuilder;
use alloy::primitives::{keccak256, Address, Bytes, B256, U256};
use alloy::providers::Provider;
use alloy::rpc::types::{TransactionInput, TransactionRequest};
use alloy::sol;
use alloy::sol_types::{SolCall, SolConstructor, SolInterface, SolValue};
use localic_utils::utils::ethereum::EthClient;
use std::error::Error;
use valence_e2e::utils::{
    ethereum::set_up_anvil_container,
    DEFAULT_ANVIL_RPC_ENDPOINT,
};
use valence_solidity_bindings::BaseAccount;

const DEPLOYER: &str = "0x4e59b44847b379578588920cA78FbF26c0B4956C";

sol! {
    // The standard CREATE2 factory typically uses this function signature
    function deploy(bytes32 salt, bytes memory code) external payable returns (address deployedAddress);
}

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

    // Extract the address from the last 20 bytes
    Address::from_slice(&hash[12..])
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(set_up_anvil_container())?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;
    let accounts = eth.get_accounts_addresses()?;

    let deployer = DEPLOYER.parse::<Address>().unwrap();

    let bytes = &BaseAccount::BYTECODE;
    println!("{}", bytes);
    let cons_call = BaseAccount::constructorCall {
        _owner: accounts[0],
        _libraries: vec![accounts[0]],
    }
    .abi_encode();

    let mut full_bytecode = bytes.to_vec();
    full_bytecode.extend_from_slice(&cons_call);

    let salt = keccak256(b"test");

    let predict_addr = deployer.create2(salt, keccak256(full_bytecode.clone()));

    // let predict_addr2 = predict_create2_address(deployer, salt, full_bytecode.as_slice());

    // let base_acc_addr =
    //     BaseAccount::constructorCall(&eth.provider, accounts[0], vec![]);

    // println!("1. BaseAccount: {:?}", t);

    // Create the transaction
    // let mut call_data = Vec::new();
    // let call_data = deployCall {
    //     salt,
    //     code: Bytes::from(full_bytecode.),
    // }.abi_encode();
    
    let mut full_data = salt.to_vec();
    full_data.extend_from_slice(&full_bytecode);

    let tx = TransactionRequest {
        to: Some(alloy::primitives::TxKind::from(deployer)),
        input: TransactionInput {
            input: Some(Bytes::from(full_data.clone())),
            data: Some(Bytes::from(full_data)),
        },
        value: Some(U256::ZERO),
        gas: Some(5000000),                  // Set an appropriate gas limit
        gas_price: Some(1_000_000_000_u128), // 1 gwei
        ..Default::default()
    }
    .from(accounts[0]);

    let res = eth.send_transaction(tx)?;

    println!("3. BaseAccount: {:?}", res);

    let base_acc = BaseAccount::new(predict_addr, &eth.provider);

    let tx = base_acc
        .approveLibrary(predict_addr)
        .into_transaction_request()
        .from(accounts[0]);
    let res = eth.send_transaction(tx)?;

    println!("4. BaseAccount: {:?}", res);

    let code = eth
        .rt
        .block_on(async { eth.provider.get_code_at(predict_addr).await })?;

    println!("Code: {:?}", code);

    // return Ok(());




    

    let approved_call = base_acc.approvedLibraries(predict_addr);
    let res = eth.rt.block_on(async { approved_call.call().await })?._0;

    let owner = base_acc.owner();
    let owner_res = eth.rt.block_on(async { owner.call().await })?._0;

    // let res = eth.send_transaction(approved)?;
    println!("5. BaseAccount: {:?}", res);
    println!("6. BaseAccount: {}", owner_res);

    let res = eth
    .rt
    .block_on(async { eth.provider.get_account(predict_addr).await })?;
    
    println!("7. BaseAccount: {:?}", res);

    Ok(())
}
