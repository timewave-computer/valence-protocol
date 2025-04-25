use std::{error::Error, str::FromStr};

use crate::{
    approve_library, strategist::strategy_config, AAVE_POOL_ADDRESS,
    CCTP_TOKEN_MESSENGER_ON_ETHEREUM, L1_STANDARD_BRIDGE_ADDRESS, USDC_ADDRESS_ON_ETHEREUM,
    WETH_ADDRESS_ON_BASE, WETH_ADDRESS_ON_ETHEREUM,
};
use alloy::{
    hex::FromHex,
    primitives::{Address, Bytes, U256},
    sol_types::SolValue,
};
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::solidity_contracts::{
    AavePositionManager, BaseAccount, CCTPTransfer, ERC1967Proxy, Forwarder,
    StandardBridgeTransfer,
    ValenceVault::{self, FeeConfig, FeeDistributionConfig, VaultConfig},
};
use valence_encoder_utils::libraries::{
    aave_position_manager::solidity_types::AavePositionManagerConfig,
    cctp_transfer::solidity_types::CCTPTransferConfig,
    forwarder::solidity_types::{ForwarderConfig, ForwardingConfig, IntervalType},
    standard_bridge_transfer::solidity_types::StandardBridgeTransferConfig,
};

pub async fn set_up_eth_accounts(
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
) -> Result<strategy_config::ethereum::EthereumAccounts, Box<dyn Error>> {
    info!("Setting up all accounts on Base");

    let mut addresses = vec![];

    for _ in 0..5 {
        let base_account_tx = BaseAccount::deploy_builder(
            &eth_client.get_request_provider().await?,
            eth_admin_addr,
            vec![],
        )
        .into_transaction_request();

        let base_account_tx = eth_client.execute_tx(base_account_tx.clone()).await?;

        let base_account_addr = base_account_tx.contract_address.unwrap();

        addresses.push(base_account_addr);
        info!(
            "Deployed BaseAccount contract at address: {:?}",
            base_account_addr
        );
    }

    let accounts = strategy_config::ethereum::EthereumAccounts {
        vault_deposit: addresses[0].to_string(),
        vault_withdraw: addresses[1].to_string(),
        aave_input: addresses[2].to_string(),
        cctp_input: addresses[3].to_string(),
        standard_bridge_input: addresses[4].to_string(),
    };

    Ok(accounts)
}

pub async fn set_up_eth_libraries(
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
    eth_strategist_addr: Address,
    eth_platform_fee_recipient: Address,
    eth_program_accounts: strategy_config::ethereum::EthereumAccounts,
    base_program_accounts: strategy_config::base::BaseAccounts,
) -> Result<strategy_config::ethereum::EthereumLibraries, Box<dyn Error>> {
    info!("Setting up all libraries on Ethereum");

    // The strategist will be the processor for simplicity
    let processor = eth_strategist_addr;

    let cctp_transfer = set_up_cctp_transfer(
        eth_client,
        Address::from_str(&eth_program_accounts.cctp_input)?,
        Address::from_str(&base_program_accounts.pancake_input)?,
        eth_admin_addr,
        processor,
    )
    .await?;

    let aave_position_manager = set_up_aave_position_manager(
        eth_client,
        Address::from_str(&eth_program_accounts.aave_input)?,
        Address::from_str(&eth_program_accounts.vault_withdraw)?,
        eth_admin_addr,
        processor,
    )
    .await?;

    let standard_bridge_transfer = set_up_standard_bridge_transfer(
        eth_client,
        Address::from_str(&eth_program_accounts.standard_bridge_input)?,
        Address::from_str(&base_program_accounts.pancake_input)?,
        eth_admin_addr,
        processor,
    )
    .await?;

    let forwarder_vault_deposit_to_aave_input = set_up_forwarder_vault_to_aave(
        eth_client,
        Address::from_str(&eth_program_accounts.vault_deposit)?,
        Address::from_str(&eth_program_accounts.aave_input)?,
        eth_admin_addr,
        processor,
    )
    .await?;

    let forwarder_vault_deposit_to_standard_bridge_input =
        set_up_forwarder_vault_to_standard_bridge(
            eth_client,
            Address::from_str(&eth_program_accounts.vault_deposit)?,
            Address::from_str(&base_program_accounts.standard_bridge_input)?,
            eth_admin_addr,
            processor,
        )
        .await?;

    let vault = set_up_vault(
        eth_client,
        Address::from_str(&eth_program_accounts.vault_deposit)?,
        Address::from_str(&eth_program_accounts.vault_withdraw)?,
        eth_strategist_addr,
        eth_platform_fee_recipient,
        eth_admin_addr,
    )
    .await?;

    let libraries = strategy_config::ethereum::EthereumLibraries {
        vault: vault.to_string(),
        cctp_transfer: cctp_transfer.to_string(),
        standard_bridge_transfer: standard_bridge_transfer.to_string(),
        aave_position_manager: aave_position_manager.to_string(),
        forwarder_vault_deposit_to_aave_input: forwarder_vault_deposit_to_aave_input.to_string(),
        forwarder_vault_deposit_to_standard_bridge_input:
            forwarder_vault_deposit_to_standard_bridge_input.to_string(),
    };

    Ok(libraries)
}

async fn set_up_vault(
    eth_client: &EthereumClient,
    deposit_account: Address,
    withdraw_account: Address,
    strategist_address: Address,
    platform_fee_recipient: Address,
    admin: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Valence Vault...");

    let fee_config = FeeConfig {
        depositFeeBps: 0,          // No deposit fee
        platformFeeBps: 10,        // 0.1 %
        performanceFeeBps: 10,     // 0.1 %
        solverCompletionFee: 0,    // No solver completion fee
    };

    let fee_distribution = FeeDistributionConfig {
        strategistAccount: strategist_address, // Strategist fee recipient
        platformAccount: platform_fee_recipient, // Platform fee recipient
        strategistRatioBps: 10,                // 0.1 % to strategist
    };
    let vault_config = VaultConfig {
        depositAccount: deposit_account,
        withdrawAccount: withdraw_account,
        strategist: strategist_address,
        fees: fee_config,
        feeDistribution: fee_distribution,
        depositCap: 0,
        withdrawLockupPeriod: 60,  // 1 minute lockup period
        maxWithdrawFeeBps: 10_000, // 1% max withdraw fee
    };

    info!("Deploying Valence Vault on Ethereum...");

    // First deploy the implementation contract
    let implementation_tx = ValenceVault::deploy_builder(&eth_client.get_request_provider().await?)
        .into_transaction_request()
        .from(admin);

    let implementation_address = eth_client
        .execute_tx(implementation_tx)
        .await?
        .contract_address
        .unwrap();

    info!("Vault deployed at: {implementation_address}");

    let proxy_tx = ERC1967Proxy::deploy_builder(
        &eth_client.get_request_provider().await?,
        implementation_address,
        Bytes::new(),
    )
    .into_transaction_request()
    .from(admin);

    let proxy_address = eth_client
        .execute_tx(proxy_tx)
        .await?
        .contract_address
        .unwrap();
    info!("Proxy deployed at: {proxy_address}");

    // Initialize the Vault
    let rp = eth_client.get_request_provider().await?;

    let vault = ValenceVault::new(proxy_address, &rp);

    let initialize_tx = vault
        .initialize(
            admin,                                                // owner
            vault_config.abi_encode().into(),                     // encoded config
            Address::from_str(WETH_ADDRESS_ON_ETHEREUM).unwrap(), // underlying token
            "Valence Test Vault".to_string(),                     // vault token name
            "vTEST".to_string(),                                  // vault token symbol
            U256::from(1e6),                                      // match deposit token precision
        )
        .into_transaction_request()
        .from(admin);

    eth_client.execute_tx(initialize_tx).await?;

    // Approve the vault on both deposit and withdraw accounts
    approve_library(eth_client, proxy_address, deposit_account).await?;
    approve_library(eth_client, proxy_address, withdraw_account).await?;

    Ok(proxy_address)
}

async fn set_up_cctp_transfer(
    eth_client: &EthereumClient,
    input_account: Address,
    mint_recipient: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up CCTP Transfer on Ethereum");

    // Convert the Address to a string, remove "0x" prefix
    let mint_recipient_string = mint_recipient.to_string();
    let mint_recipient_hex = mint_recipient_string.strip_prefix("0x").unwrap_or_default();

    // Create the padded hex string
    let padded_hex = format!("{:0>64}", mint_recipient_hex);

    let cctp_transer_config = CCTPTransferConfig {
        amount: U256::ZERO,
        mintRecipient: alloy_primitives_encoder::FixedBytes::<32>::from_hex(padded_hex)?,
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        destinationDomain: 6,
        cctpTokenMessenger: alloy_primitives_encoder::Address::from_str(
            CCTP_TOKEN_MESSENGER_ON_ETHEREUM,
        )?,
        transferToken: alloy_primitives_encoder::Address::from_str(USDC_ADDRESS_ON_ETHEREUM)?,
    };

    let cctp_tx = CCTPTransfer::deploy_builder(
        &eth_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&cctp_transer_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = eth_client.execute_tx(cctp_tx).await?;

    let cctp_transfer_address = response.contract_address.unwrap();
    info!("CCTP Transfer deployed at: {cctp_transfer_address}");

    // Approve the vault on input account
    approve_library(eth_client, cctp_transfer_address, input_account).await?;

    Ok(cctp_transfer_address)
}

async fn set_up_aave_position_manager(
    eth_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Aave Position Manager on Ethereum");

    let aave_position_manager_config = AavePositionManagerConfig {
        poolAddress: alloy_primitives_encoder::Address::from_str(AAVE_POOL_ADDRESS)?,
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        supplyAsset: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_ETHEREUM)?,
        borrowAsset: alloy_primitives_encoder::Address::from_str(USDC_ADDRESS_ON_ETHEREUM)?,
        referralCode: 0,
    };

    let aave_position_manager_tx = AavePositionManager::deploy_builder(
        &eth_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&aave_position_manager_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = eth_client.execute_tx(aave_position_manager_tx).await?;
    let aave_position_manager_address = response.contract_address.unwrap();

    info!("Aave Position Manager deployed at: {aave_position_manager_address}");

    // Approve the vault on input account
    approve_library(eth_client, aave_position_manager_address, input_account).await?;

    Ok(aave_position_manager_address)
}

async fn set_up_standard_bridge_transfer(
    eth_client: &EthereumClient,
    input_account: Address,
    recipient: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Standard Bridge Transfer on Ethereum");

    let standard_bridge_transfer_config = StandardBridgeTransferConfig {
        amount: U256::ZERO,
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        recipient: alloy_primitives_encoder::Address::from_str(recipient.to_string().as_str())?,
        standardBridge: alloy_primitives_encoder::Address::from_str(L1_STANDARD_BRIDGE_ADDRESS)?,
        token: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_ETHEREUM)?,
        remoteToken: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_BASE)?,
        minGasLimit: 200000,
        extraData: alloy_primitives_encoder::Bytes::new(),
    };

    let standard_bridge_transfer_tx = StandardBridgeTransfer::deploy_builder(
        &eth_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&standard_bridge_transfer_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = eth_client.execute_tx(standard_bridge_transfer_tx).await?;
    let standard_bridge_transfer_address = response.contract_address.unwrap();

    info!("Standard Bridge Transfer deployed at: {standard_bridge_transfer_address}");

    // Approve the vault on input account
    approve_library(eth_client, standard_bridge_transfer_address, input_account).await?;

    Ok(standard_bridge_transfer_address)
}

async fn set_up_forwarder_vault_to_aave(
    eth_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Forwarder Vault to Aave on Ethereum");

    let forwarder_vault_to_aave_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        // Strategist will update this to forward the right amount
        forwardingConfigs: vec![ForwardingConfig {
            tokenAddress: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_ETHEREUM)?,
            maxAmount: 0,
        }],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };

    let forwarder_vault_to_aave_tx = Forwarder::deploy_builder(
        &eth_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&forwarder_vault_to_aave_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = eth_client.execute_tx(forwarder_vault_to_aave_tx).await?;
    let forwarder_vault_to_aave_address = response.contract_address.unwrap();

    info!("Forwarder Vault to Aave deployed at: {forwarder_vault_to_aave_address}");

    // Approve the vault on input account
    approve_library(eth_client, forwarder_vault_to_aave_address, input_account).await?;

    Ok(forwarder_vault_to_aave_address)
}

async fn set_up_forwarder_vault_to_standard_bridge(
    eth_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Forwarder Vault to Standard Bridge on Ethereum");

    let forwarder_vault_to_standard_bridge_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        // Strategist will update this to forward the right amount
        forwardingConfigs: vec![ForwardingConfig {
            tokenAddress: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_ETHEREUM)?,
            maxAmount: 0,
        }],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };

    let forwarder_vault_to_standard_bridge_tx = Forwarder::deploy_builder(
        &eth_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&forwarder_vault_to_standard_bridge_config)
            .into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = eth_client
        .execute_tx(forwarder_vault_to_standard_bridge_tx)
        .await?;
    let forwarder_vault_to_standard_bridge_address = response.contract_address.unwrap();

    info!(
        "Forwarder Vault to Standard Bridge deployed at: {forwarder_vault_to_standard_bridge_address}"
    );

    // Approve the vault on input account
    approve_library(
        eth_client,
        forwarder_vault_to_standard_bridge_address,
        input_account,
    )
    .await?;

    Ok(forwarder_vault_to_standard_bridge_address)
}
