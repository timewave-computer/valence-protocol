use std::{error::Error, str::FromStr, time::SystemTime};

use alloy::primitives::Address;
use evm::{setup_eth_accounts, setup_eth_libraries};
use localic_utils::{
    utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
};

use log::info;

use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
};

use valence_e2e::{
    async_run,
    utils::{
        authorization::set_up_authorization_and_processor,
        ethereum::{
            self as ethereum_utils, set_up_anvil_container, ANVIL_NAME, DEFAULT_ANVIL_PORT,
        },
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        solidity_contracts::{BaseAccount, MockERC20},
        DEFAULT_ANVIL_RPC_ENDPOINT, ETHEREUM_CHAIN_NAME, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
    },
};

const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_vault/neutron_contracts/";

mod evm;
mod strategist;

const WBTC_ERC20: &str = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599";
const WBTC_WHALE: &str = "0x0555E30da8f98308EdB960aa94C0Db47230d2B9c";
const EUREKA_HANDLER: &str = "0xfc2d0487a0ae42ae7329a80dc269916a9184cf7c";
const EUREKA_HANDLER_SRC_CLIENT: &str = "cosmoshub-0";
const WBTC_NEUTRON_DENOM: &str = "WBTC";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // first we work the eth mainnet and set up an anvil env with it
    let fork_url = "https://eth-mainnet.public.blastapi.io";
    set_up_anvil_container(ANVIL_NAME, DEFAULT_ANVIL_PORT, Some(fork_url))
        .await
        .unwrap();

    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();

    let rt = tokio::runtime::Runtime::new()?;

    let eth_rp = eth_client.get_request_provider().await.unwrap();

    let eth_accounts = eth_client.get_provider_accounts().await.unwrap();

    let eth_admin_acc = eth_accounts[0];
    let _eth_user_acc = eth_accounts[2];
    let strategist_acc = Address::from_str("0x14dc79964da2c08b23698b3d3cc7ca32193d9955").unwrap();

    let admin_balance = eth_client
        .query_balance(&eth_admin_acc.to_string())
        .await
        .unwrap();

    info!("admin balance: {:?}", admin_balance);

    let deposit_account_init_tx =
        BaseAccount::deploy_builder(&eth_rp, eth_admin_acc, vec![]).into_transaction_request();
    let withdraw_account_init_tx =
        BaseAccount::deploy_builder(&eth_rp, eth_admin_acc, vec![]).into_transaction_request();

    let deposit_account_rx = eth_client
        .execute_tx(deposit_account_init_tx.clone())
        .await
        .unwrap();
    let withdraw_account_rx = eth_client
        .execute_tx(withdraw_account_init_tx.clone())
        .await
        .unwrap();

    let deposit_account_addr = deposit_account_rx.contract_address.unwrap();
    let withdraw_account_addr = withdraw_account_rx.contract_address.unwrap();

    info!("deposit account address: {deposit_account_addr}");
    info!("withdraw account address: {withdraw_account_addr}");

    let wbtc_token_address = Address::from_str(WBTC_ERC20).unwrap();
    let eureka_handler_address = Address::from_str(EUREKA_HANDLER).unwrap();
    let wbtc_whale_address = Address::from_str(WBTC_WHALE).unwrap();

    let wbtc_contract = MockERC20::new(wbtc_token_address, eth_rp);

    let whale_wbtc_balance = eth_client
        .query(wbtc_contract.balanceOf(wbtc_whale_address))
        .await?;

    info!("wbtc whale balance: {:?}", whale_wbtc_balance._0);

    // spin up the testctx with only neutron
    // let mut test_ctx = async_run!(
    //     rt,
    //     TestContextBuilder::default()
    //         .with_unwrap_raw_logs(true)
    //         .with_api_url(LOCAL_IC_API_URL)
    //         .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
    //         .with_chain(ConfigChainBuilder::default_neutron().build().unwrap())
    //         .with_log_file_path(LOGS_FILE_PATH)
    //         .build()
    // )?;

    let (neutron_grpc_url, neutron_grpc_port) = get_grpc_address_and_port_from_url(
        &get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?,
    )?;

    let neutron_client = NeutronClient::new(
        &neutron_grpc_url,
        &neutron_grpc_port,
        "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry",
        NEUTRON_CHAIN_ID,
    )
    .await?;

    // mint the mock wbtc on neutron
    // test_ctx
    //     .build_tx_mint_tokenfactory_token()
    //     .with_denom(WBTC_NEUTRON_DENOM)
    //     .with_amount(100_000_000_000)
    //     .send()?;

    // async_run!(rt, std::thread::sleep(std::time::Duration::from_secs(3)););

    let neutron_admin_wbtc_balance = neutron_client
        .query_balance(NEUTRON_CHAIN_ADMIN_ADDR, WBTC_NEUTRON_DENOM)
        .await?;

    info!("neutron admin wbtc balance: {neutron_admin_wbtc_balance}");

    rt.shutdown_background();

    Ok(())
}
