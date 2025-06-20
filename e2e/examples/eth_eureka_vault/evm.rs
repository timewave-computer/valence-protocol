use std::{error::Error, str::FromStr};

use alloy::primitives::{Address, U256};
use log::info;
use valence_domain_clients::{
    clients::ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::{
    async_run,
    utils::{
        solidity_contracts::{
            IBCEurekaTransfer,
            ValenceVault::{FeeConfig, FeeDistributionConfig, VaultConfig},
        },
        vault::setup_valence_vault,
    },
};
use valence_encoder_utils::libraries::ibc_eureka_transfer::solidity_types::IBCEurekaTransferConfig;

use crate::strategist::{routing::query_skip_eureka_route, strategy_config};

pub fn setup_eth_accounts(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
) -> Result<strategy_config::ethereum::EthereumAccounts, Box<dyn Error>> {
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

    let accounts = strategy_config::ethereum::EthereumAccounts {
        deposit: deposit_acc_addr.to_string(),
        withdraw: withdraw_acc_addr.to_string(),
    };

    Ok(accounts)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn setup_eth_libraries(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
    eth_strategist_addr: Address,
    eth_program_accounts: strategy_config::ethereum::EthereumAccounts,
    eth_accounts: &[Address],
    eth_hyperlane_mailbox_addr: String,
    ntrn_authorizations_addr: String,
    wbtc_token_address: Address,
    neutron_deposit_account: String,
) -> Result<strategy_config::ethereum::EthereumLibraries, Box<dyn Error>> {
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

    info!("Setting up Eureka forwarder on Ethereum");
    let eureka_forwarder = setup_eureka_forwarder(
        rt,
        eth_client,
        eth_admin_addr,
        eth_strategist_addr, // should be processor
        wbtc_token_address,
        Address::from_str(&eth_program_accounts.deposit).unwrap(),
        neutron_deposit_account,
    )?;

    let fee_config = FeeConfig {
        depositFeeBps: 0,       // No deposit fee
        platformFeeBps: 10,     // 0.1% yearly platform fee
        performanceFeeBps: 10,  // 0.1% performance fee
        solverCompletionFee: 0, // No solver completion fee
    };

    let fee_distribution = FeeDistributionConfig {
        strategistAccount: eth_accounts[0], // Strategist fee recipient
        platformAccount: eth_accounts[1],   // Platform fee recipient
        strategistRatioBps: 10,             // 0.1% to strategist
    };

    let vault_config = VaultConfig {
        depositAccount: Address::from_str(&eth_program_accounts.deposit).unwrap(),
        withdrawAccount: Address::from_str(&eth_program_accounts.withdraw).unwrap(),
        strategist: eth_strategist_addr,
        fees: fee_config,
        feeDistribution: fee_distribution,
        depositCap: 0, // No cap (for real)
        withdrawLockupPeriod: 1,
        // withdrawLockupPeriod: SECONDS_IN_DAY, // 1 day lockup
        maxWithdrawFeeBps: 10_000, // 1% max withdraw fee
    };

    let vault_address = setup_valence_vault(
        rt,
        eth_client,
        eth_admin_addr,
        eth_program_accounts.deposit,
        eth_program_accounts.withdraw,
        wbtc_token_address,
        vault_config,
        1e8,
    )?;

    let libraries = strategy_config::ethereum::EthereumLibraries {
        lite_processor: lite_processor_address.to_string(),
        valence_vault: vault_address.to_string(),
        eureka_transfer: eureka_forwarder.to_string(),
    };

    Ok(libraries)
}

/// sets up the Eureka transfer library to route funds from Ethereum
/// to Neutron
#[allow(clippy::too_many_arguments)]
pub fn setup_eureka_forwarder(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    admin: Address,
    processor: Address,
    transfer_token: Address,
    input_acc: Address,
    recipient: String,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    info!("deploying Eureka transfer lib on Ethereum...");

    let inner_rt = tokio::runtime::Runtime::new()?;
    let skip_api_response = async_run!(
        inner_rt,
        query_skip_eureka_route(
            "1",
            "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
            "cosmoshub-4",
            "ibc/D742E8566B0B8CC8F569D950051C09CF57988A88F0E45574BFB3079D41DE6462",
            "100000000".to_string(),
        )
        .await
    )
    .unwrap();
    let relative_timeout_secs = skip_api_response.timeout / 1_000_000_000;

    info!("skip api evm route query response: {:?}", skip_api_response);

    let cfg = IBCEurekaTransferConfig {
        amount: U256::ZERO,
        minAmountOut: U256::ZERO,
        transferToken: alloy_primitives_encoder::Address::from_str(
            transfer_token.to_string().as_str(),
        )?,
        inputAccount: alloy_primitives_encoder::Address::from_str(input_acc.to_string().as_str())?,
        recipient,
        sourceClient: skip_api_response.source_client,
        timeout: relative_timeout_secs,
        eurekaHandler: alloy_primitives_encoder::Address::from_str(
            &skip_api_response.entry_contract_address,
        )?,
    };

    let eureka_tx = IBCEurekaTransfer::deploy_builder(
        eth_rp,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&cfg).into(),
    )
    .into_transaction_request()
    .from(admin);

    let eureka_rx = async_run!(rt, eth_client.execute_tx(eureka_tx).await.unwrap());

    let eureka_transfer_address = eureka_rx.contract_address.unwrap();

    info!("IBC Eureka transfer deployed at: {eureka_transfer_address}");

    valence_e2e::utils::ethereum::valence_account::approve_library(
        rt,
        eth_client,
        input_acc,
        eureka_transfer_address,
    );

    Ok(eureka_transfer_address)
}
