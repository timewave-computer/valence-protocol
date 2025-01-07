use std::error::Error;

use local_interchaintest::utils::ethereum::EthClient;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    
    let eth = EthClient::new("http://localhost:8545")?;
    
    let block = eth.get_block_number()?;
    println!("Current block number: {}", block);

    Ok(())
}