use std::{error::Error, str::FromStr};

use alloy::{
    primitives::{Address, U256},
    sol_types::SolValue,
};
use log::{info, warn};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient as _, request_provider_client::RequestProviderClient},
};

use crate::utils::{
    solidity_contracts::{
        BaseAccount, LiteProcessor,
        MockERC20::{self},
        ValenceVault::{self, FeeConfig, FeeDistributionConfig, VaultConfig},
    },
    NEUTRON_HYPERLANE_DOMAIN,
};

// use crate::SECONDS_IN_DAY;

/// macro for executing async code in a blocking context
macro_rules! async_run {
    ($rt:expr, $($body:tt)*) => {
        $rt.block_on(async { $($body)* })
    }
}

pub fn vault_update(
    vault_addr: Address,
    new_rate: U256,
    withdraw_fee_bps: u32,
    netting_amount: U256,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<(), Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());
    let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

    let config = async_run!(rt, eth_client.query(valence_vault.config()).await.unwrap());
    info!("pre-update vault config: {:?}", config);

    let start_rate = async_run!(rt, {
        eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
            ._0
    });
    let prev_max_rate = async_run!(rt, {
        eth_client
            .query(valence_vault.maxHistoricalRate())
            .await
            .unwrap()
            ._0
    });

    let prev_total_assets = async_run!(rt, {
        eth_client
            .query(valence_vault.totalAssets())
            .await
            .unwrap()
            ._0
    });

    info!("Vault start rate: {start_rate}");
    info!("Vault current max historical rate: {prev_max_rate}");
    info!("Vault current total assets: {prev_total_assets}");
    info!(
            "Updating vault with new rate: {new_rate}, withdraw fee: {withdraw_fee_bps}bps, netting: {netting_amount}"
        );

    let update_result = async_run!(rt, {
        eth_client
            .execute_tx(
                valence_vault
                    .update(new_rate, withdraw_fee_bps, netting_amount)
                    .into_transaction_request(),
            )
            .await
    });

    if let Err(e) = &update_result {
        info!("Update failed: {:?}", e);
        panic!("failed to update vault");
    }

    let new_redemption_rate = async_run!(rt, {
        eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
            ._0
    });
    let new_max_historical_rate = async_run!(rt, {
        eth_client
            .query(valence_vault.maxHistoricalRate())
            .await
            .unwrap()
            ._0
    });

    let new_total_assets = async_run!(rt, {
        eth_client
            .query(valence_vault.totalAssets())
            .await
            .unwrap()
            ._0
    });

    let config = async_run!(rt, eth_client.query(valence_vault.config()).await.unwrap());
    info!("Vault new config: {:?}", config);
    info!("Vault new redemption rate: {new_redemption_rate}");
    info!("Vault new max historical rate: {new_max_historical_rate}");
    info!("Vault new total assets: {new_total_assets}");

    assert_eq!(
        new_redemption_rate, new_rate,
        "Redemption rate should be updated to the new rate"
    );

    // Verify max historical rate was updated if needed
    if new_rate > prev_max_rate {
        assert_eq!(
            new_max_historical_rate, new_rate,
            "Max historical rate should be updated when new rate is higher"
        );
    } else {
        assert_eq!(
            new_max_historical_rate, prev_max_rate,
            "Max historical rate should remain unchanged when new rate is not higher"
        );
    }

    Ok(())
}

pub fn setup_vault_config(
    accounts: &[Address],
    eth_deposit_acc: Address,
    eth_withdraw_acc: Address,
) -> VaultConfig {
    let fee_config = FeeConfig {
        depositFeeBps: 0,        // No deposit fee
        platformFeeBps: 1000,    // 10% yearly platform fee
        performanceFeeBps: 2000, // 20% performance fee
        solverCompletionFee: 0,  // No solver completion fee
    };

    let fee_distribution = FeeDistributionConfig {
        strategistAccount: accounts[0], // Strategist fee recipient
        platformAccount: accounts[1],   // Platform fee recipient
        strategistRatioBps: 5000,       // 50% to strategist
    };

    VaultConfig {
        depositAccount: eth_deposit_acc,
        withdrawAccount: eth_withdraw_acc,
        strategist: accounts[0],
        fees: fee_config,
        feeDistribution: fee_distribution,
        depositCap: 0, // No cap (for real)
        withdrawLockupPeriod: 1,
        // withdrawLockupPeriod: SECONDS_IN_DAY, // 1 day lockup
        maxWithdrawFeeBps: 100, // 1% max withdraw fee
    }
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

        let client = eth_client.get_request_provider().await.unwrap();
        let valence_vault = ValenceVault::new(vault_addr, &client);

        let signed_tx = valence_vault
            .deposit(amount, user)
            .into_transaction_request()
            .from(user);

        match alloy::providers::Provider::send_transaction(&client, signed_tx).await {
            Ok(resp) => {
                let tx_hash = resp.get_receipt().await?.transaction_hash;
                info!("deposit completed: {:?}", tx_hash);
            }
            Err(e) => warn!("failed to deposit into vault: {:?}", e),
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
                let receipt = resp.get_receipt().await.unwrap();
                info!("complete withdrawal request receipt: {:?}", receipt);
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

pub fn setup_deposit_erc20(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, { eth_client.get_request_provider().await.unwrap() });

    info!("Deploying ERC20s on Ethereum...");
    let evm_vault_deposit_token_tx =
        MockERC20::deploy_builder(&eth_rp, "TestUSDC".to_string(), "TUSD".to_string())
            .into_transaction_request();

    let evm_vault_deposit_token_rx = async_run!(rt, {
        valence_chain_client_utils::evm::base_client::EvmBaseClient::execute_tx(
            eth_client,
            evm_vault_deposit_token_tx,
        )
        .await
        .unwrap()
    });

    let valence_vault_deposit_token_address = evm_vault_deposit_token_rx.contract_address.unwrap();

    Ok(valence_vault_deposit_token_address)
}

pub fn setup_valence_account(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    admin: Address,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, { eth_client.get_request_provider().await.unwrap() });

    info!("Deploying base account on Ethereum...");
    let base_account_tx =
        BaseAccount::deploy_builder(&eth_rp, admin, vec![]).into_transaction_request();

    let base_account_rx = async_run!(rt, {
        eth_client
            .execute_tx(base_account_tx.clone())
            .await
            .unwrap()
    });

    let base_account_addr = base_account_rx.contract_address.unwrap();

    Ok(base_account_addr)
}

pub fn setup_lite_processor(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    admin: Address,
    mailbox: &str,
    authorization_contract_address: &str,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, { eth_client.get_request_provider().await.unwrap() });

    let tx = LiteProcessor::deploy_builder(
        &eth_rp,
        crate::utils::hyperlane::bech32_to_evm_bytes32(authorization_contract_address)?,
        Address::from_str(mailbox)?,
        NEUTRON_HYPERLANE_DOMAIN,
        vec![admin],
    )
    .into_transaction_request();

    let lite_processor_rx = async_run!(rt, { eth_client.execute_tx(tx).await.unwrap() });

    let lite_processor_address = lite_processor_rx.contract_address.unwrap();
    info!("Lite Processor deployed at: {}", lite_processor_address);

    Ok(lite_processor_address)
}

pub fn setup_valence_vault(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_accounts: &[Address],
    admin: Address,
    eth_deposit_acc: Address,
    eth_withdraw_acc: Address,
    vault_deposit_token_addr: Address,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, { eth_client.get_request_provider().await.unwrap() });

    info!("deploying Valence Vault on Ethereum...");
    let vault_config = setup_vault_config(eth_accounts, eth_deposit_acc, eth_withdraw_acc);

    let vault_tx = ValenceVault::deploy_builder(
        &eth_rp,
        admin,                            // owner
        vault_config.abi_encode().into(), // encoded config
        vault_deposit_token_addr,         // underlying token
        "Valence Test Vault".to_string(), // vault token name
        "vTEST".to_string(),              // vault token symbol
        U256::from(1e18),                 // placeholder, tbd what a reasonable value should be here
    )
    .into_transaction_request();

    let vault_rx = async_run!(rt, { eth_client.execute_tx(vault_tx).await.unwrap() });

    let vault_address = vault_rx.contract_address.unwrap();
    info!("Vault deployed at: {vault_address}");

    Ok(vault_address)
}
