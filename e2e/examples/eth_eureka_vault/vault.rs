use std::{
    error::Error,
    str::FromStr,
    time::{Duration, SystemTime},
};

use alloy::primitives::Address;

use evm::{setup_eth_accounts, setup_eth_libraries};
use localic_utils::{
    utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_ID,
};

use log::{info, warn};

use program::{setup_neutron_libraries, upload_neutron_contracts};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
    neutron::NeutronClient,
};

mod program;

use valence_e2e::{
    async_run,
    utils::{
        astroport::setup_astroport_cl_pool,
        authorization::set_up_authorization_and_processor,
        ethereum::{set_up_anvil_container, ANVIL_NAME, DEFAULT_ANVIL_PORT},
        parse::{get_chain_field_from_local_ic_log, get_grpc_address_and_port_from_url},
        solidity_contracts::{MockERC20, ValenceVault},
        DEFAULT_ANVIL_RPC_ENDPOINT, LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
    },
};

const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";

mod evm;
mod strategist;

const WBTC_ERC20: &str = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599";
const WBTC_WHALE: &str = "0x0555E30da8f98308EdB960aa94C0Db47230d2B9c";
const EUREKA_HANDLER: &str = "0xfc2d0487a0ae42ae7329a80dc269916a9184cf7c";
const EUREKA_HANDLER_SRC_CLIENT: &str = "cosmoshub-0";
const WBTC_NEUTRON_DENOM: &str = "WBTC";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_eureka_vault/neutron_contracts/";

// #[tokio::main]
fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let rt = tokio::runtime::Runtime::new()?;

    // first we work the eth mainnet and set up an anvil env with it
    let fork_url = "https://eth-mainnet.public.blastapi.io";

    async_run!(
        rt,
        set_up_anvil_container(ANVIL_NAME, DEFAULT_ANVIL_PORT, Some(fork_url))
            .await
            .unwrap()
    );

    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let eth_accounts = async_run!(rt, eth_client.get_provider_accounts().await.unwrap());
    let eth_admin_acc = eth_accounts[0];

    let ethereum_program_accounts = setup_eth_accounts(&rt, &eth_client, eth_admin_acc)?;

    let eth_admin_acc = eth_accounts[0];
    let _eth_user_acc = eth_accounts[2];
    let strategist_acc = Address::from_str("0x14dc79964da2c08b23698b3d3cc7ca32193d9955").unwrap();

    let admin_balance = async_run!(
        rt,
        eth_client
            .query_balance(&eth_admin_acc.to_string())
            .await
            .unwrap()
    );

    info!("admin balance: {:?}", admin_balance);

    let wbtc_token_address = Address::from_str(WBTC_ERC20).unwrap();
    let eureka_handler_address = Address::from_str(EUREKA_HANDLER).unwrap();
    let wbtc_whale_address = Address::from_str(WBTC_WHALE).unwrap();

    let wbtc_contract = MockERC20::new(wbtc_token_address, eth_rp);

    let whale_wbtc_balance = async_run!(
        rt,
        eth_client
            .query(wbtc_contract.balanceOf(wbtc_whale_address))
            .await
    )?;

    info!("wbtc whale balance: {:?}", whale_wbtc_balance._0);

    let (neutron_grpc_url, neutron_grpc_port) = get_grpc_address_and_port_from_url(
        &get_chain_field_from_local_ic_log(NEUTRON_CHAIN_ID, "grpc_address")?,
    )?;

    info!("neutron grpc: {neutron_grpc_url}");
    info!("neutron grpc port: {neutron_grpc_port}");

    let neutron_client = async_run!(rt, NeutronClient::new(
        &neutron_grpc_url,
        &neutron_grpc_port,
        "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry",
        NEUTRON_CHAIN_ID,
    )
    .await)?;

    async_run!(rt, tokio::time::sleep(Duration::from_secs(3)).await);
    info!("neutron client ready!");

    async_run!(
        rt,
        match neutron_client
            .create_tokenfactory_denom(WBTC_NEUTRON_DENOM)
            .await
        {
            Ok(tf_create_rx) => {
                neutron_client
                    .poll_for_tx(&tf_create_rx.hash)
                    .await
                    .unwrap();
            }
            Err(e) => warn!("tokenfactory denom already exists: {:?}", e),
        }
    );

    let wbtc_on_neutron = format!("factory/{NEUTRON_CHAIN_ADMIN_ADDR}/WBTC");

    async_run!(rt, {
        let tf_mint_rx = neutron_client
            .mint_tokenfactory_tokens(
                WBTC_NEUTRON_DENOM,
                100_000_000_000,
                Some(NEUTRON_CHAIN_ADMIN_ADDR),
            )
            .await
            .unwrap();
        neutron_client.poll_for_tx(&tf_mint_rx.hash).await.unwrap();
        let neutron_admin_wbtc_balance = neutron_client
            .query_balance(NEUTRON_CHAIN_ADMIN_ADDR, &wbtc_on_neutron)
            .await
            .unwrap();

        info!("neutron admin wbtc balance: {neutron_admin_wbtc_balance}");
    });

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build().unwrap())
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    // setup astroport
    let (pool_addr, lp_token) =
        setup_astroport_cl_pool(&mut test_ctx, wbtc_on_neutron.to_string())?;

    info!("BTC-NTRN cl pool: {:?}", pool_addr);

    // set up the authorization and processor contracts on neutron
    let (authorization_contract_address, neutron_processor_address) =
        set_up_authorization_and_processor(
            &mut test_ctx,
            hex::encode(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs()
                    .to_string(),
            ),
        )?;

    upload_neutron_contracts(&mut test_ctx)?;

    let neutron_program_accounts = program::setup_neutron_accounts(&mut test_ctx)?;

    let amount_to_transfer = 1_000_000;

    let neutron_program_libraries = setup_neutron_libraries(
        &mut test_ctx,
        &neutron_program_accounts,
        &pool_addr,
        &authorization_contract_address,
        &neutron_processor_address,
        amount_to_transfer,
        &wbtc_on_neutron,
        ethereum_program_accounts.withdraw.to_string(),
        &lp_token,
    )?;

    let ethereum_program_libraries = setup_eth_libraries(
        &rt,
        &eth_client,
        eth_admin_acc,
        strategist_acc,
        ethereum_program_accounts.clone(),
        &eth_accounts,
        "hyperlane_mock".to_string(),
        authorization_contract_address,
        wbtc_token_address,
        neutron_program_accounts.deposit,
        EUREKA_HANDLER_SRC_CLIENT.to_string(),
        eureka_handler_address,
    )?;

    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let vault_address = Address::from_str(&ethereum_program_libraries.valence_vault).unwrap();
    let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    rt.shutdown_background();

    Ok(())
}
