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

#[allow(unused)]
pub fn log_eth_balances(
    eth_client: &EthereumClient,
    rt: &tokio::runtime::Runtime,
    vault_addr: &Address,
    vault_deposit_token: &Address,
    deposit_acc_addr: &Address,
    withdraw_acc_addr: &Address,
    depositor_addr: &Address,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
        let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

        let (
            depositor_usdc_bal,
            depositor_vault_bal,
            withdraw_acc_usdc_bal,
            deposit_acc_usdc_bal,
            vault_total_supply,
        ) = tokio::join!(
            eth_client.query(usdc_token.balanceOf(*depositor_addr)),
            eth_client.query(valence_vault.balanceOf(*depositor_addr)),
            eth_client.query(usdc_token.balanceOf(*withdraw_acc_addr)),
            eth_client.query(usdc_token.balanceOf(*deposit_acc_addr)),
            eth_client.query(valence_vault.totalSupply()),
        );

        let depositor_usdc_bal = depositor_usdc_bal.unwrap()._0;
        let depositor_vault_bal = depositor_vault_bal.unwrap()._0;
        let withdraw_acc_usdc_bal = withdraw_acc_usdc_bal.unwrap()._0;
        let deposit_acc_usdc_bal = deposit_acc_usdc_bal.unwrap()._0;
        let vault_total_supply = vault_total_supply.unwrap()._0;

        info!("USER SHARES\t\t: {depositor_vault_bal}");
        info!("USER USDC\t\t: {depositor_usdc_bal}");
        info!("WITHDRAW ACC USDC\t: {withdraw_acc_usdc_bal}");
        info!("DEPOSIT ACC USDC\t: {deposit_acc_usdc_bal}");
        info!("VAULT TOTAL SUPPLY\t: {vault_total_supply}");
    });

    Ok(())
}
