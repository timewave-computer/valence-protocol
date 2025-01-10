use std::error::Error;

use alloy::{network::TransactionBuilder, primitives::U256, rpc::types::TransactionRequest};
use local_interchaintest::utils::{
    ethereum::EthClient, solidity_contracts::BaseAccount, DEFAULT_ANVIL_RPC_ENDPOINT,
    HYPERLANE_COSMWASM_ARTIFACTS_PATH, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH,
    VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let block = eth.get_block_number()?;
    println!("Current block number: {}", block);

    let accounts = eth.get_accounts_addresses()?;
    println!("Accounts: {:?}", accounts);

    // Get balance
    let balance = eth.get_balance(accounts[0])?;
    println!("Balance: {} wei", balance);

    let account = eth.get_account(accounts[0])?;
    println!("Account 0: {:?}", account);

    let balance_account_0_before = eth.get_balance(accounts[0])?;
    println!("Balance account 0 before: {} wei", balance_account_0_before);
    let balance_account_1_before = eth.get_balance(accounts[1])?;
    println!("Balance account 1 before: {} wei", balance_account_1_before);

    let tx = TransactionRequest::default()
        .from(accounts[0])
        .to(accounts[1])
        .with_value(U256::from(100));
    let receipt = eth.send_transaction(tx)?;
    println!("Transaction hash: {}", receipt.transaction_hash);

    let balance_account_0_after = eth.get_balance(accounts[0])?;
    println!("Balance account 0 after: {} wei", balance_account_0_after);
    let balance_account_1_after = eth.get_balance(accounts[1])?;
    println!("Balance account 1 after: {} wei", balance_account_1_after);

    let tx = eth.get_transaction_by_hash(receipt.transaction_hash)?;
    println!("Transaction: {:?}", tx);

    let transaction = BaseAccount::deploy_builder(&eth.provider, accounts[0], vec![])
        .into_transaction_request()
        .from(accounts[0]);

    let contract_address = eth.send_transaction(transaction)?.contract_address.unwrap();
    println!("Contract Address: {:?}", contract_address);

    let contract = BaseAccount::new(contract_address, &eth.provider);

    let builder = contract.owner();
    let owner = eth.rt.block_on(async { builder.call().await })?._0;
    println!("Owner: {:?}", owner);

    let builder = contract.approveLibrary(accounts[1]);
    let tx = builder.into_transaction_request().from(accounts[0]);
    eth.send_transaction(tx)?;

    // Check that approved libraries was updated
    let builder = contract.approvedLibraries(accounts[1]);
    let approved_libraries = eth.rt.block_on(async { builder.call().await })?._0;
    println!("Approved Libraries: {:?}", approved_libraries);

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // Upload all Hyperlane contracts to Neutron
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .send_with_local_cache(
            HYPERLANE_COSMWASM_ARTIFACTS_PATH,
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )
        .unwrap();

    Ok(())
}
