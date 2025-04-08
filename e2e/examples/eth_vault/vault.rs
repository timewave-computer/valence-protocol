use std::{
    error::Error,
    str::FromStr,
    thread::sleep,
    time::{Duration, SystemTime},
};

use alloy::primitives::{Address, U256};
use evm::{setup_eth_accounts, setup_eth_libraries, EthereumUsers};
use localic_utils::{
    types::config::ConfigChain, utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};

use log::info;
use neutron::setup_astroport_cl_pool;
use program::{setup_neutron_accounts, setup_neutron_libraries, upload_neutron_contracts};

use strategist::client::Strategist;
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient, evm::request_provider_client::RequestProviderClient,
};

use valence_e2e::{
    async_run,
    utils::{
        authorization::set_up_authorization_and_processor,
        ethereum as ethereum_utils, mock_cctp_relayer,
        solidity_contracts::ValenceVault,
        vault::{self},
        DEFAULT_ANVIL_RPC_ENDPOINT, LOGS_FILE_PATH, NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM,
        NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME, NOBLE_CHAIN_PREFIX, UUSDC_DENOM, VALENCE_ARTIFACTS_PATH,
    },
};

const _PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const _WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_vault/neutron_contracts/";

mod evm;
mod neutron;
mod noble;
mod program;
mod strategist;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;

    info!("Initializing ethereum side flow...");
    async_run!(rt, ethereum_utils::set_up_anvil_container().await)?;

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

    let ethereum_program_accounts = setup_eth_accounts(&rt, &eth_client, eth_admin_acc)?;

    // set up the cctp messenger
    let mock_cctp_messenger_address = evm::setup_mock_token_messenger(&rt, &eth_client)?;
    // eth side USDC token
    let usdc_token_address =
        ethereum_utils::mock_erc20::setup_deposit_erc20(&rt, &eth_client, "MockUSDC", "USDC")?;

    info!("Setting up Neutron side flow...");

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChain {
            denom: NOBLE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: NOBLE_CHAIN_ID.to_string(),
            chain_name: NOBLE_CHAIN_NAME.to_string(),
            chain_prefix: NOBLE_CHAIN_PREFIX.to_string(),
            admin_addr: NOBLE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, NOBLE_CHAIN_NAME)
        .build()?;

    // setup hyperlane between neutron and eth
    let program_hyperlane_contracts = utils::hyperlane_plumbing(&mut test_ctx, &eth)?;

    let uusdc_on_neutron_denom = test_ctx
        .get_ibc_denom()
        .base_denom(UUSDC_DENOM.to_owned())
        .src(NOBLE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let noble_client = noble::get_client(&rt)?;
    noble::setup_environment(&rt, &noble_client)?;
    noble::mint_usdc_to_addr(&rt, &noble_client, NOBLE_CHAIN_ADMIN_ADDR, 999900000)?;

    async_run!(&rt, {
        let rx = noble_client
            .ibc_transfer(
                NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
                UUSDC_DENOM.to_string(),
                999000000.to_string(),
                test_ctx
                    .get_transfer_channels()
                    .src(NOBLE_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
                60,
                None,
            )
            .await
            .unwrap();
        noble_client.poll_for_tx(&rx.hash).await.unwrap();
    });

    sleep(Duration::from_secs(3));

    // setup astroport
    let (pool_addr, lp_token) =
        setup_astroport_cl_pool(&mut test_ctx, uusdc_on_neutron_denom.to_string())?;

    let amount_to_transfer = 1_000_000;

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

    let neutron_program_accounts = setup_neutron_accounts(&mut test_ctx)?;

    let neutron_program_libraries = setup_neutron_libraries(
        &mut test_ctx,
        &neutron_program_accounts,
        &pool_addr,
        &neutron_processor_address,
        amount_to_transfer,
        &uusdc_on_neutron_denom,
        ethereum_program_accounts.withdraw.to_string(),
        &lp_token,
    )?;

    let ethereum_program_libraries = setup_eth_libraries(
        &rt,
        &eth_client,
        eth_admin_acc,
        strategist_acc,
        ethereum_program_accounts.clone(),
        mock_cctp_messenger_address,
        usdc_token_address,
        neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string(),
        program_hyperlane_contracts
            .eth_hyperlane_contracts
            .mailbox
            .to_string(),
        authorization_contract_address,
        &eth_accounts,
    )?;

    let valence_vault = ValenceVault::new(ethereum_program_libraries.valence_vault, &eth_rp);

    let user_1_deposit_amount = U256::from(5_000_000);
    let user_2_deposit_amount = U256::from(1_000_000);

    let mut eth_users =
        EthereumUsers::new(usdc_token_address, ethereum_program_libraries.valence_vault);
    eth_users.add_user(&rt, &eth_client, eth_accounts[2]);
    eth_users.fund_user(&rt, &eth_client, 0, user_1_deposit_amount);
    eth_users.add_user(&rt, &eth_client, eth_accounts[3]);
    eth_users.fund_user(&rt, &eth_client, 1, user_2_deposit_amount);

    info!("Starting CCTP mock relayer between Noble and Ethereum...");
    let mock_cctp_relayer = mock_cctp_relayer::MockCctpRelayer::new(
        &rt,
        mock_cctp_messenger_address,
        usdc_token_address,
    )?;
    let rly_rt = tokio::runtime::Runtime::new().unwrap();

    let _join_handle = rly_rt.spawn(mock_cctp_relayer.start());
    info!("main sleep for 3...");
    sleep(Duration::from_secs(3));

    let strategist = Strategist::new(
        &rt,
        neutron_program_accounts.clone(),
        neutron_program_libraries.clone(),
        ethereum_program_accounts.clone(),
        uusdc_on_neutron_denom.clone(),
        lp_token.to_string(),
        pool_addr.to_string(),
        ethereum_program_libraries.cctp_forwarder,
        ethereum_program_libraries.valence_vault,
        usdc_token_address,
    )
    .unwrap();

    info!("User depositing {user_1_deposit_amount}USDC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_users.users[0],
        user_1_deposit_amount,
    )?;

    evm::mine_blocks(&rt, &eth_client, 5, 3);

    let strategist_rt = tokio::runtime::Runtime::new().unwrap();
    let _strategist_join_handle = strategist_rt.spawn(strategist.start());

    // giving the strategist some time to process the deposits
    for _ in 1..5 {
        info!("main sleep for 4sec, mining evm blocks...");
        evm::mine_blocks(&rt, &eth_client, 5, 3);
        sleep(Duration::from_secs(4));
    }

    info!("User2 depositing {user_2_deposit_amount}USDC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_users.users[1],
        U256::from(1_000_000),
    )?;

    // giving the strategist some time to process the deposits
    for _ in 1..5 {
        info!("main sleep for 4sec, mining evm blocks...");
        evm::mine_blocks(&rt, &eth_client, 5, 3);
        sleep(Duration::from_secs(4));
    }

    let user1_pre_redeem_shares_bal = eth_users.get_user_shares(&rt, &eth_client, 0);
    info!("USER1 initiating the redeem of {user1_pre_redeem_shares_bal} shares from vault...");
    vault::redeem(
        ethereum_program_libraries.valence_vault,
        &rt,
        &eth_client,
        eth_users.users[0],
        user1_pre_redeem_shares_bal,
        10_000,
        true,
    )?;

    // giving the strategist some time to process the withdraw request
    for _ in 1..10 {
        info!("main sleep for 4sec, mining evm blocks...");
        evm::mine_blocks(&rt, &eth_client, 5, 3);
        sleep(Duration::from_secs(4));
    }

    let pre_completion_user0_bal = eth_users.get_user_usdc(&rt, &eth_client, 0);

    async_run!(
        &rt,
        eth_users
            .log_balances(
                &eth_client,
                &ethereum_program_libraries.valence_vault,
                &usdc_token_address,
            )
            .await
    );

    info!("User0 completing withdraw request...");
    vault::complete_withdraw_request(
        ethereum_program_libraries.valence_vault,
        &rt,
        &eth_client,
        eth_users.users[0],
    )?;

    async_run!(
        &rt,
        eth_users
            .log_balances(
                &eth_client,
                &ethereum_program_libraries.valence_vault,
                &usdc_token_address,
            )
            .await
    );

    let post_completion_user0_bal = eth_users.get_user_usdc(&rt, &eth_client, 0);

    assert!(
        pre_completion_user0_bal < post_completion_user0_bal,
        "user failed to complete withdrawal"
    );

    for _ in 1..5 {
        info!("main sleep for 4sec, mining evm blocks...");
        evm::mine_blocks(&rt, &eth_client, 5, 3);
        sleep(Duration::from_secs(4));
    }

    Ok(())
}
