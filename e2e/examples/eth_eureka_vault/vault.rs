use std::{error::Error, str::FromStr};

use alloy::primitives::Address;
use localic_utils::utils::ethereum::EthClient;

use log::info;

use valence_chain_client_utils::evm::request_provider_client::RequestProviderClient;

use valence_e2e::{
    async_run,
    utils::{
        ethereum::{self as ethereum_utils, ANVIL_NAME, DEFAULT_ANVIL_PORT},
        DEFAULT_ANVIL_RPC_ENDPOINT,
    },
};

const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_vault/neutron_contracts/";

mod strategist;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;

    info!("Initializing ethereum side flow...");
    async_run!(
        rt,
        ethereum_utils::set_up_anvil_container(ANVIL_NAME, DEFAULT_ANVIL_PORT, None).await
    )?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let eth_accounts = async_run!(rt, eth_client.get_provider_accounts().await.unwrap());
    let eth_admin_acc = eth_accounts[0];
    let _eth_user_acc = eth_accounts[2];
    let strategist_acc = Address::from_str("0x14dc79964da2c08b23698b3d3cc7ca32193d9955").unwrap();

    Ok(())
}
