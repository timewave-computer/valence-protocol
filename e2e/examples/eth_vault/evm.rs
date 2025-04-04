use std::error::Error;

use alloy::primitives::{Address, U256};
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};

use valence_e2e::{
    async_run,
    utils::{
        solidity_contracts::{
            MockERC20,
            ValenceVault::{self},
        },
        vault::setup_cctp_transfer,
    },
};

pub fn mine_blocks(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    blocks: usize,
    interval: usize,
) {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        alloy::providers::ext::AnvilApi::anvil_mine(
            &eth_rp,
            Some(U256::from(blocks)),
            Some(U256::from(interval)),
        )
        .await
        .unwrap();
    });
}

#[allow(clippy::too_many_arguments)]
pub fn log_eth_balances(
    eth_client: &EthereumClient,
    rt: &tokio::runtime::Runtime,
    vault_addr: &Address,
    vault_deposit_token: &Address,
    deposit_acc_addr: &Address,
    withdraw_acc_addr: &Address,
    user1_addr: &Address,
    user2_addr: &Address,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
        let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

        let (
            user1_usdc_bal,
            user2_usdc_bal,
            user1_vault_bal,
            user2_vault_bal,
            withdraw_acc_usdc_bal,
            deposit_acc_usdc_bal,
            vault_total_supply,
        ) = tokio::join!(
            eth_client.query(usdc_token.balanceOf(*user1_addr)),
            eth_client.query(usdc_token.balanceOf(*user2_addr)),
            eth_client.query(valence_vault.balanceOf(*user1_addr)),
            eth_client.query(valence_vault.balanceOf(*user2_addr)),
            eth_client.query(usdc_token.balanceOf(*withdraw_acc_addr)),
            eth_client.query(usdc_token.balanceOf(*deposit_acc_addr)),
            eth_client.query(valence_vault.totalSupply()),
        );

        let user1_usdc_bal = user1_usdc_bal.unwrap()._0;
        let user2_usdc_bal = user2_usdc_bal.unwrap()._0;
        let user1_vault_bal = user1_vault_bal.unwrap()._0;
        let user2_vault_bal = user2_vault_bal.unwrap()._0;
        let withdraw_acc_usdc_bal = withdraw_acc_usdc_bal.unwrap()._0;
        let deposit_acc_usdc_bal = deposit_acc_usdc_bal.unwrap()._0;
        let vault_total_supply = vault_total_supply.unwrap()._0;

        info!("USER1 SHARES\t\t: {user1_vault_bal}");
        info!("USER1 USDC\t\t: {user1_usdc_bal}");
        info!("USER2 SHARES\t\t: {user2_vault_bal}");
        info!("USER2 USDC\t\t: {user2_usdc_bal}");
        info!("WITHDRAW ACC USDC\t: {withdraw_acc_usdc_bal}");
        info!("DEPOSIT ACC USDC\t: {deposit_acc_usdc_bal}");
        info!("VAULT TOTAL SUPPLY\t: {vault_total_supply}");
    });

    Ok(())
}

#[derive(Clone, Debug)]
pub struct EthereumProgramLibraries {
    pub cctp_forwarder: Address,
    pub lite_processor: Address,
    pub valence_vault: Address,
}

#[derive(Clone, Debug)]
pub struct EthereumProgramAccounts {
    pub deposit: Address,
    pub withdraw: Address,
}

pub fn setup_eth_accounts(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
) -> Result<EthereumProgramAccounts, Box<dyn Error>> {
    info!("Setting up Deposit and Withdraw accounts on Ethereum");

    // create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    let deposit_acc_addr = valence_e2e::utils::ethereum::valence_account::setup_valence_account(
        rt,
        eth_client,
        eth_admin_addr,
    )?;
    let withdraw_acc_addr = valence_e2e::utils::ethereum::valence_account::setup_valence_account(
        rt,
        eth_client,
        eth_admin_addr,
    )?;

    let accounts = EthereumProgramAccounts {
        deposit: deposit_acc_addr,
        withdraw: withdraw_acc_addr,
    };

    Ok(accounts)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_eth_libraries(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
    eth_strategist_addr: Address,
    deposit_acc_addr: Address,
    withdraw_acc_addr: Address,
    cctp_messenger_addr: Address,
    usdc_token_addr: Address,
    noble_inbound_ica_addr: String,
    eth_hyperlane_mailbox_addr: String,
    ntrn_authorizations_addr: String,
    eth_accounts: &[Address],
) -> Result<EthereumProgramLibraries, Box<dyn Error>> {
    info!("Setting up CCTP Transfer on Ethereum");
    let cctp_forwarder_addr = setup_cctp_transfer(
        rt,
        eth_client,
        noble_inbound_ica_addr,
        deposit_acc_addr,
        eth_admin_addr,
        eth_strategist_addr,
        usdc_token_addr,
        cctp_messenger_addr,
    )?;

    info!("Setting up Lite Processor on Ethereum");
    let lite_processor_address =
        valence_e2e::utils::ethereum::lite_processor::setup_lite_processor(
            rt,
            eth_client,
            eth_admin_addr,
            &eth_hyperlane_mailbox_addr,
            &ntrn_authorizations_addr,
        )?;

    info!("Setting up Valence Vault...");
    let vault_address = valence_e2e::utils::vault::setup_valence_vault(
        rt,
        eth_client,
        eth_strategist_addr,
        eth_accounts,
        eth_admin_addr,
        deposit_acc_addr,
        withdraw_acc_addr,
        usdc_token_addr,
    )?;

    let libraries = EthereumProgramLibraries {
        cctp_forwarder: cctp_forwarder_addr,
        lite_processor: lite_processor_address,
        valence_vault: vault_address,
    };

    Ok(libraries)
}
