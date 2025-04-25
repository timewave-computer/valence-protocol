use std::error::Error;

use ethereum::setup_eth_accounts;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::ethereum::{set_up_anvil_container, ANVIL_NAME, DEFAULT_ANVIL_PORT};

const ETH_FORK_URL: &str = "https://eth-mainnet.public.blastapi.io";
const ETH_ANVIL_PORT: &str = "1337";
const BASE_FORK_URL: &str = "https://mainnet.base.org";
const BASE_ANVIL_PORT: &str = "1338";
const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
pub const WETH_ADDRESS_ON_ETHEREUM: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

mod ethereum;
mod strategist;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up the Anvil container for Ethereum
    set_up_anvil_container("anvil_ethereum", ETH_ANVIL_PORT, Some(ETH_FORK_URL))
        .await
        .unwrap();

    // Set up the Anvil container for Base
    set_up_anvil_container("anvil_base", BASE_ANVIL_PORT, Some(BASE_FORK_URL))
        .await
        .unwrap();

    // Create an Ethereum client
    let eth_client = EthereumClient::new(
        format!("http://127.0.0.1:{}", ETH_ANVIL_PORT).as_str(),
        TEST_MNEMONIC,
    )
    .unwrap();

    // Create a Base client
    let base_client = EthereumClient::new(
        format!("http://127.0.0.1:{}", BASE_ANVIL_PORT).as_str(),
        TEST_MNEMONIC,
    )
    .unwrap();

    // Get an admin account
    let accounts_eth = eth_client.get_provider_accounts().await.unwrap();
    let eth_admin_addr = accounts_eth[0];

    let ethereum_program_accounts = setup_eth_accounts(&eth_client, eth_admin_acc)?;

    Ok(())
}
