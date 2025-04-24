use std::error::Error;

use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::ethereum::{set_up_anvil_container, ANVIL_NAME, DEFAULT_ANVIL_PORT};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let fork_url = "https://eth-mainnet.public.blastapi.io";
    set_up_anvil_container("anvil2", "8546", Some(fork_url))
        .await
        .unwrap();

    let client = EthereumClient::new(
        "http://127.0.0.1:8546",
        "test test test test test test test test test test test junk",
    )
    .unwrap();
    let accounts = client.get_provider_accounts().await.unwrap();

    let balance = client
        .query_balance(&accounts[0].to_string())
        .await
        .unwrap();
    println!("Balance: {:?}", balance);

    // Query balance of vitalik account
    let vitalik_address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    let vitalik_balance = client.query_balance(vitalik_address).await.unwrap();
    println!("Vitalik Balance: {:?}", vitalik_balance);

    set_up_anvil_container(ANVIL_NAME, DEFAULT_ANVIL_PORT, None)
        .await
        .unwrap();

    let second_client = EthereumClient::new(
        "http://127.0.0.1:8545",
        "test test test test test test test test test test test junk",
    )
    .unwrap();

    let second_accounts = second_client.get_provider_accounts().await.unwrap();
    let second_balance = second_client
        .query_balance(&second_accounts[0].to_string())
        .await
        .unwrap();
    println!("Second Client Balance: {:?}", second_balance);

    Ok(())
}
