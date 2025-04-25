use std::{error::Error, str::FromStr};

use alloy::primitives::Address;
use base::set_up_base_accounts;
use ethereum::set_up_eth_accounts;
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{ethereum::set_up_anvil_container, solidity_contracts::BaseAccount};

const ETH_FORK_URL: &str = "https://eth-mainnet.public.blastapi.io";
const ETH_ANVIL_PORT: &str = "1337";
const BASE_FORK_URL: &str = "https://mainnet.base.org";
const BASE_ANVIL_PORT: &str = "1338";
const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
pub const WETH_ADDRESS_ON_ETHEREUM: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
pub const WETH_ADDRESS_ON_BASE: &str = "0x4200000000000000000000000000000000000006";
pub const USDC_ADDRESS_ON_ETHEREUM: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const USDC_ADDRESS_ON_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
pub const CCTP_TOKEN_MESSENGER_ON_ETHEREUM: &str = "0xBd3fa81B58Ba92a82136038B25aDec7066af3155";
pub const CCTP_TOKEN_MESSENGER_ON_BASE: &str = "0x1682Ae6375C4E4A97e4B583BC394c861A46D8962";
pub const AAVE_POOL_ADDRESS: &str = "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2";
pub const L1_STANDARD_BRIDGE_ADDRESS: &str = "0x3154Cf16ccdb4C6d922629664174b904d80F2C35";
pub const L2_STANDARD_BRIDGE_ADDRESS: &str = "0x4200000000000000000000000000000000000010";
pub const PANCAKE_POSITION_MANAGER_ON_BASE: &str = "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364";
pub const PANCAKE_MASTERCHEF_ON_BASE: &str = "0xC6A2Db661D5a5690172d8eB0a7DEA2d3008665A3";

mod base;
mod ethereum;
mod strategist;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up the Anvil container for Ethereum
    set_up_anvil_container("anvil_ethereum", ETH_ANVIL_PORT, Some(ETH_FORK_URL)).await?;

    // Set up the Anvil container for Base
    set_up_anvil_container("anvil_base", BASE_ANVIL_PORT, Some(BASE_FORK_URL)).await?;

    // Create an Ethereum client
    let eth_client = EthereumClient::new(
        format!("http://127.0.0.1:{}", ETH_ANVIL_PORT).as_str(),
        TEST_MNEMONIC,
    )?;

    // Create a Base client
    let base_client = EthereumClient::new(
        format!("http://127.0.0.1:{}", BASE_ANVIL_PORT).as_str(),
        TEST_MNEMONIC,
    )?;

    let strategist_acc = Address::from_str("0x14dc79964da2c08b23698b3d3cc7ca32193d9955").unwrap();

    // Get an admin account for Ethereum
    let accounts_eth = eth_client.get_provider_accounts().await?;
    let eth_admin_addr = accounts_eth[0];

    // Create all the acounts needed for Ethereum
    let ethereum_accounts = set_up_eth_accounts(&eth_client, eth_admin_addr).await?;

    // Get an admin account for Base
    let accounts_base = base_client.get_provider_accounts().await?;
    let base_admin_addr = accounts_base[0];

    // Create all the accounts needed for Base
    let base_accounts = set_up_base_accounts(&base_client, base_admin_addr).await?;

    // Set up ethereum libraries
    let ethereum_libraries = ethereum::set_up_eth_libraries(
        &eth_client,
        accounts_eth[0], // admin
        strategist_acc,  // strategist
        strategist_acc,  // platform fee receiver
        ethereum_accounts.clone(),
        base_accounts.clone(),
    )
    .await?;

    info!(
        "Ethereum libraries set up successfully: {:?}",
        ethereum_libraries
    );

    // Set up base libraries
    let base_libraries = base::set_up_base_libraries(
        &base_client,
        accounts_base[0], // admin
        strategist_acc,   // strategist
        base_accounts.clone(),
        ethereum_accounts.clone(),
    )
    .await?;

    info!("Base libraries set up successfully: {:?}", base_libraries);

    Ok(())
}

// Helper function to approve a library from a Base Account
pub async fn approve_library(
    client: &EthereumClient,
    library: Address,
    account: Address,
) -> Result<(), Box<dyn Error>> {
    let rp = client.get_request_provider().await?;

    // Approve the library on the account
    info!("Approving library {} on account {}...", library, account);
    let base_account = BaseAccount::new(account, &rp);

    client
        .execute_tx(
            base_account
                .approveLibrary(library)
                .into_transaction_request(),
        )
        .await?;

    Ok(())
}
