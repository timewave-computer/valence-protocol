use std::{error::Error, str::FromStr};

use alloy::primitives::Address;
use local_interchaintest::utils::ethereum::EthClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let eth = EthClient::new("http://localhost:8545")?;

    let block = eth.get_block_number()?;
    println!("Current block number: {}", block);

    // Address to check (this is the first Anvil test account)
    let address = Address::from_str("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266")?;

    // Get balance
    let balance = eth.get_balance(address)?;
    println!("Balance: {} wei", balance);

    Ok(())
}
