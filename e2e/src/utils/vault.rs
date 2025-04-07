use std::error::Error;

use alloy::primitives::{Address, U256};
use log::{info, warn};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};

use crate::{
    async_run,
    utils::solidity_contracts::ValenceVault::{self},
};

pub fn query_vault_packed_values(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::packedValuesReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.packedValues())
            .await
            .unwrap()
    })
}

pub fn pause(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        match eth_client
            .execute_tx(valence_vault.pause().into_transaction_request())
            .await
        {
            Ok(_) => info!("vault paused!"),
            Err(_) => warn!("failed to pause the vault!"),
        };

        let packed_vals = eth_client
            .query(valence_vault.packedValues())
            .await
            .unwrap();

        assert!(packed_vals.paused, "vault should be paused");
    });

    Ok(())
}

pub fn unpause(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);
        match eth_client
            .execute_tx(valence_vault.unpause().into_transaction_request())
            .await
        {
            Ok(_) => info!("vault resumed!"),
            Err(_) => warn!("failed to resume the vault!"),
        };
        let packed_vals = eth_client
            .query(valence_vault.packedValues())
            .await
            .unwrap();

        assert!(!packed_vals.paused, "vault should be unpaused");
    });

    Ok(())
}

pub fn deposit_to_vault(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    vault_addr: Address,
    user: Address,
    amount: U256,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        info!("user depositing {amount} into vault...");

        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        let signed_tx = valence_vault
            .deposit(amount, user)
            .into_transaction_request()
            .from(user);

        match alloy::providers::Provider::send_transaction(&eth_rp, signed_tx).await {
            Ok(resp) => {
                let tx_hash = resp.get_receipt().await?.transaction_hash;
                info!("deposit completed: {:?}", tx_hash);
            }
            Err(e) => {
                warn!("failed to deposit into vault: {:?}", e)
            }
        };

        Ok(())
    })
}

pub fn query_vault_config(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::configReturn {
    let config = async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client.query(valence_vault.config()).await.unwrap()
    });
    info!("VAULT CONFIG config: {:?}", config);
    config
}

pub fn query_vault_total_assets(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::totalAssetsReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client.query(valence_vault.totalAssets()).await.unwrap()
    })
}

pub fn query_vault_total_supply(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::totalSupplyReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client.query(valence_vault.totalSupply()).await.unwrap()
    })
}

pub fn query_redemption_rate(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> ValenceVault::redemptionRateReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
    })
}

pub fn query_vault_balance_of(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> ValenceVault::balanceOfReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.balanceOf(addr))
            .await
            .unwrap()
    })
}

pub fn addr_has_active_withdraw(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> ValenceVault::hasActiveWithdrawReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.hasActiveWithdraw(addr))
            .await
            .unwrap()
    })
}

pub fn addr_withdraw_request(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> ValenceVault::userWithdrawRequestReturn {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        eth_client
            .query(valence_vault.userWithdrawRequest(addr))
            .await
            .unwrap()
    })
}

pub fn complete_withdraw_request(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let client = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &client);

        let signed_tx = valence_vault
            .completeWithdraw(addr)
            .into_transaction_request()
            .from(addr);

        match alloy::providers::Provider::send_transaction(&client, signed_tx).await {
            Ok(resp) => {
                resp.get_receipt().await.unwrap();
                info!("withdrawal complete!");
            }
            Err(e) => warn!("complete withdrawal request error: {:?}", e),
        };
        Ok(())
    })
}

pub fn redeem(
    vault_addr: Address,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    addr: Address,
    amount: U256,
    max_loss_bps: u32,
    allow_solver_completion: bool,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let client = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &client);
        let signed_tx = valence_vault
            .redeem_0(amount, addr, addr, max_loss_bps, allow_solver_completion)
            .into_transaction_request()
            .from(addr);
        match alloy::providers::Provider::send_transaction(&client, signed_tx).await {
            Ok(resp) => {
                let receipt = resp.get_receipt().await.unwrap();
                info!("redeem request response: {:?}", receipt.transaction_hash);
            }
            Err(e) => warn!("redeem request error: {:?}", e),
        };
        Ok(())
    })
}

pub fn update() -> Result<(), Box<dyn Error>> {
    // query both neutron and eth sides
    // find netting amount
    // update
    Ok(())
}
