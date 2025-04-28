use std::{collections::BTreeMap, error::Error};

use alloy::primitives::{Address, U256};
use cosmwasm_std::Uint128;
use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, GAIA_CHAIN_NAME, NEUTRON_CHAIN_ADMIN_ADDR,
    NEUTRON_CHAIN_NAME,
};
use log::{info, warn};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_forwarder_library::msg::{ForwardingConstraints, UncheckedForwardingConfig};
use valence_generic_ibc_transfer_library::msg::IbcTransferAmount;
use valence_library_utils::{denoms::UncheckedDenom, LibraryAccountType};

use crate::{
    async_run,
    utils::{
        base_account::approve_library,
        manager::{FORWARDER_NAME, NEUTRON_IBC_TRANSFER_NAME},
        solidity_contracts::ValenceVault::{self},
    },
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
                let receipt = resp.get_receipt().await.unwrap();
                info!(
                    "withdrawal complete! receipt hash: {:?}",
                    receipt.transaction_hash
                );
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

pub fn setup_liquidation_fwd_lib(
    test_ctx: &mut TestContext,
    input_account: String,
    output_addr: String,
    shares_denom: &str,
) -> Result<String, Box<dyn Error>> {
    let fwd_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get(FORWARDER_NAME)
        .unwrap();

    let fwd_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_forwarder_library::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: valence_forwarder_library::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(input_account.clone()),
            output_addr: LibraryAccountType::Addr(output_addr.clone()),
            forwarding_configs: vec![UncheckedForwardingConfig {
                denom: UncheckedDenom::Native(shares_denom.to_string()),
                max_amount: Uint128::MAX,
            }],
            forwarding_constraints: ForwardingConstraints::new(None),
        },
    };

    info!(
        "Neutron Forwarder instantiate message: {:?}",
        fwd_instantiate_msg
    );

    let liquidation_forwarder = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        fwd_code_id,
        &serde_json::to_string(&fwd_instantiate_msg).unwrap(),
        "liquidation_forwarder",
        None,
        "",
    )
    .unwrap();

    info!(
        "Liquidation Forwarder library: {}",
        liquidation_forwarder.address.clone()
    );

    // Approve the library for the base account
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        liquidation_forwarder.address.clone(),
        None,
    );

    Ok(liquidation_forwarder.address)
}

pub fn setup_neutron_ibc_transfer_lib(
    test_ctx: &mut TestContext,
    input_account: String,
    output_addr: String,
    denom: &str,
    _authorizations: String,
    _processor: String,
) -> Result<String, Box<dyn Error>> {
    let neutron_ibc_transfer_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get(NEUTRON_IBC_TRANSFER_NAME)
        .unwrap();

    let neutron_ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_neutron_ibc_transfer_library::msg::LibraryConfig,
    > {
        // TODO: uncomment to not bypass authorizations/processor logic
        // owner: authorizations.to_string(),
        // processor: processor.to_string(),
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: valence_neutron_ibc_transfer_library::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(input_account.to_string()),
            amount: IbcTransferAmount::FullAmount,
            denom: valence_library_utils::denoms::UncheckedDenom::Native(denom.to_string()),
            remote_chain_info: valence_generic_ibc_transfer_library::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN_NAME)
                    .dest(GAIA_CHAIN_NAME) // TODO: review if this is correct for eureka route to eth
                    .get(),
                ibc_transfer_timeout: None,
            },
            output_addr: LibraryAccountType::Addr(output_addr.to_string()),
            memo: "-".to_string(),
            denom_to_pfm_map: BTreeMap::default(),
            eureka_config: None,
        },
    };

    info!(
        "Neutron IBC Transfer instantiate message: {:?}",
        neutron_ibc_transfer_instantiate_msg
    );

    let ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ibc_transfer_code_id,
        &serde_json::to_string(&neutron_ibc_transfer_instantiate_msg).unwrap(),
        "neutron_ibc_transfer",
        None,
        "",
    )
    .unwrap();

    info!(
        "Neutron IBC Transfer library: {}",
        ibc_transfer.address.clone()
    );

    // Approve the library for the base account
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account,
        ibc_transfer.address.clone(),
        None,
    );

    Ok(ibc_transfer.address)
}
