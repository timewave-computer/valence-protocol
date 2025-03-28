use std::{error::Error, str::FromStr};

use alloy::{
    hex::FromHex,
    primitives::{Address, Bytes, U256},
    providers::ext::AnvilApi,
    sol_types::SolValue,
};
use log::{info, warn};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_encoder_utils::libraries::cctp_transfer::solidity_types::CCTPTransferConfig;

use crate::{
    async_run,
    utils::solidity_contracts::{
        CCTPTransfer, ERC1967Proxy, MockTokenMessenger,
        ValenceVault::{self, FeeConfig, FeeDistributionConfig, VaultConfig},
    },
};

pub fn vault_update(
    vault_addr: Address,
    new_rate: U256,
    withdraw_fee_bps: u32,
    netting_amount: U256,
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        eth_rp
            .anvil_mine(Some(U256::from(5)), Some(U256::from(1)))
            .await
            .unwrap();

        let valence_vault = ValenceVault::new(vault_addr, &eth_rp);

        let config = eth_client.query(valence_vault.config()).await.unwrap();
        info!("pre-update vault config: {:?}", config);

        let start_rate = eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
            ._0;
        let prev_max_rate = eth_client
            .query(valence_vault.maxHistoricalRate())
            .await
            .unwrap()
            ._0;

        let prev_total_assets = eth_client
            .query(valence_vault.totalAssets())
            .await
            .unwrap()
            ._0;

        info!("Vault start rate: {start_rate}");
        info!("Vault current max historical rate: {prev_max_rate}");
        info!("Vault current total assets: {prev_total_assets}");
        info!(
                "Updating vault with new rate: {new_rate}, withdraw fee: {withdraw_fee_bps}bps, netting: {netting_amount}"
            );

        let update_result = eth_client
            .execute_tx(
                valence_vault
                    .update(new_rate, withdraw_fee_bps, netting_amount)
                    .into_transaction_request(),
            )
            .await;

        if let Err(e) = &update_result {
            info!("Update failed: {:?}", e);
            panic!("failed to update vault");
        }

        let new_redemption_rate = eth_client
            .query(valence_vault.redemptionRate())
            .await
            .unwrap()
            ._0;
        let new_max_historical_rate = eth_client
            .query(valence_vault.maxHistoricalRate())
            .await
            .unwrap()
            ._0;

        let new_total_assets = eth_client
            .query(valence_vault.totalAssets())
            .await
            .unwrap()
            ._0;

        let config = eth_client.query(valence_vault.config()).await.unwrap();
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
    })
}

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

pub fn setup_valence_vault(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_accounts: &[Address],
    admin: Address,
    eth_deposit_acc: Address,
    eth_withdraw_acc: Address,
    vault_deposit_token_addr: Address,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    info!("deploying Valence Vault on Ethereum...");
    let vault_config = setup_vault_config(eth_accounts, eth_deposit_acc, eth_withdraw_acc);

    // First deploy the implementation contract
    let implementation_tx = ValenceVault::deploy_builder(&eth_rp)
        .into_transaction_request()
        .from(admin);

    let implementation_address = async_run!(
        rt,
        eth_client
            .execute_tx(implementation_tx)
            .await
            .unwrap()
            .contract_address
            .unwrap()
    );

    info!("Vault deployed at: {implementation_address}");

    let proxy_address = async_run!(rt, {
        // Deploy the proxy contract
        let proxy_tx = ERC1967Proxy::deploy_builder(&eth_rp, implementation_address, Bytes::new())
            .into_transaction_request()
            .from(admin);

        let proxy_address = eth_client
            .execute_tx(proxy_tx)
            .await
            .unwrap()
            .contract_address
            .unwrap();
        info!("Proxy deployed at: {proxy_address}");
        proxy_address
    });

    // Initialize the Vault
    let vault = ValenceVault::new(proxy_address, &eth_rp);

    async_run!(rt, {
        let initialize_tx = vault
            .initialize(
                admin,                            // owner
                vault_config.abi_encode().into(), // encoded config
                vault_deposit_token_addr,         // underlying token
                "Valence Test Vault".to_string(), // vault token name
                "vTEST".to_string(),              // vault token symbol
                U256::from(1e18), // placeholder, tbd what a reasonable value should be here
            )
            .into_transaction_request()
            .from(admin);

        eth_client.execute_tx(initialize_tx).await.unwrap();
    });

    Ok(proxy_address)
}

pub fn setup_mock_token_messenger(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    info!("deploying Mock Token Messenger lib on Ethereum...");

    let messenger_tx = MockTokenMessenger::deploy_builder(eth_rp).into_transaction_request();

    let messenger_rx = async_run!(rt, eth_client.execute_tx(messenger_tx).await.unwrap());

    let messenger_address = messenger_rx.contract_address.unwrap();
    info!("Mock CCTP Token Messenger deployed at: {messenger_address}");

    Ok(messenger_address)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_cctp_transfer(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    noble_recipient: String,
    input_account: Address,
    admin: Address,
    processor: Address,
    usdc_token_address: Address,
    cctp_token_messenger_address: Address,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    info!("deploying CCTP Transfer lib on Ethereum...");

    // Decode the bech32 address
    let (_, data) = bech32::decode(&noble_recipient)?;
    // Convert to hex
    let address_hex = hex::encode(data);
    // Pad with zeroes to 32 bytes
    let padded_hex = format!("{:0>64}", address_hex);

    let cctp_transer_cfg = CCTPTransferConfig {
        amount: U256::from(1_000_000),
        mintRecipient: alloy_primitives_encoder::FixedBytes::<32>::from_hex(padded_hex)?,
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        destinationDomain: 4,
        cctpTokenMessenger: alloy_primitives_encoder::Address::from_str(
            cctp_token_messenger_address.to_string().as_str(),
        )?,
        transferToken: alloy_primitives_encoder::Address::from_str(
            usdc_token_address.to_string().as_str(),
        )?,
    };

    let cctp_tx = CCTPTransfer::deploy_builder(
        &eth_rp,
        admin,
        admin,
        alloy_sol_types_encoder::SolValue::abi_encode(&cctp_transer_cfg).into(),
    )
    .into_transaction_request()
    .from(admin);

    let cctp_rx = async_run!(rt, eth_client.execute_tx(cctp_tx).await.unwrap());

    let cctp_address = cctp_rx.contract_address.unwrap();
    info!("CCTP Transfer deployed at: {cctp_address}");

    Ok(cctp_address)
}
