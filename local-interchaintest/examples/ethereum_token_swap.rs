use std::error::Error;

use alloy::{network::TransactionBuilder, primitives::U256, rpc::types::TransactionRequest};
use local_interchaintest::utils::ethereum::EthClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let eth = EthClient::new("http://localhost:8545")?;

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
    let hash = eth.send_transaction(tx)?;
    println!("Transaction hash: {}", hash);

    let balance_account_0_after = eth.get_balance(accounts[0])?;
    println!("Balance account 0 after: {} wei", balance_account_0_after);
    let balance_account_1_after = eth.get_balance(accounts[1])?;
    println!("Balance account 1 after: {} wei", balance_account_1_after);

    Ok(())
}
