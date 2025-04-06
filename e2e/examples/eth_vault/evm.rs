use std::error::Error;

use alloy::primitives::Address;
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};

use valence_e2e::{
    async_run,
    utils::solidity_contracts::{
        MockERC20,
        ValenceVault::{self},
    },
};

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
