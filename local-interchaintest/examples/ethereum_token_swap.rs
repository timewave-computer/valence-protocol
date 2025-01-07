use std::error::Error;

use alloy_primitives::U256;
use alloy_rpc_client::RpcClient;
use alloy_transport_http::{reqwest::Url, Http};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Create a single-threaded runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Setup client
    let url = Url::parse("http://localhost:8545").expect("Invalid URL");
    let transport = Http::new(url);
    let client = RpcClient::new(transport, true);

    // Use runtime to block on async calls
    let block_number: U256 = rt
        .block_on(client.request("eth_blockNumber", ()))
        .expect("Could not get block number");
    println!("Current block number: {}", block_number);

    Ok(())
}
