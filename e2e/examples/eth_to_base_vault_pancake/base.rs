use std::{error::Error, str::FromStr};

use crate::{
    approve_library, strategist::strategy_config, CCTP_TOKEN_MESSENGER_ON_BASE,
    L2_STANDARD_BRIDGE_ADDRESS, PANCAKE_MASTERCHEF_ON_BASE, PANCAKE_POSITION_MANAGER_ON_BASE,
    USDC_ADDRESS_ON_BASE, WETH_ADDRESS_ON_BASE, WETH_ADDRESS_ON_ETHEREUM,
};
use alloy::{
    hex::FromHex,
    primitives::{Address, U256},
};
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::solidity_contracts::{
    BaseAccount, CCTPTransfer, Forwarder, PancakeSwapV3PositionManager, StandardBridgeTransfer,
};
use valence_encoder_utils::libraries::{
    cctp_transfer::solidity_types::CCTPTransferConfig,
    forwarder::solidity_types::{ForwarderConfig, ForwardingConfig, IntervalType},
    pancake_v3_position_manager::solidity_types::PancakeSwapV3PositionManagerConfig,
    standard_bridge_transfer::solidity_types::StandardBridgeTransferConfig,
};

pub async fn set_up_base_accounts(
    base_client: &EthereumClient,
    eth_admin_addr: Address,
) -> Result<strategy_config::base::BaseAccounts, Box<dyn Error>> {
    info!("Setting up all accounts on Ethereum");

    let mut addresses = vec![];

    for _ in 0..4 {
        let base_account_tx = BaseAccount::deploy_builder(
            &base_client.get_request_provider().await?,
            eth_admin_addr,
            vec![],
        )
        .into_transaction_request();

        let base_account_tx = base_client.execute_tx(base_account_tx.clone()).await?;

        let base_account_addr = base_account_tx.contract_address.unwrap();

        addresses.push(base_account_addr);
        info!(
            "Deployed BaseAccount contract at address: {:?}",
            base_account_addr
        );
    }

    let accounts = strategy_config::base::BaseAccounts {
        pancake_input: addresses[0].to_string(),
        pancake_output: addresses[1].to_string(),
        cctp_input: addresses[2].to_string(),
        standard_bridge_input: addresses[3].to_string(),
    };

    Ok(accounts)
}

pub async fn set_up_base_libraries(
    base_client: &EthereumClient,
    base_admin_addr: Address,
    base_strategist_addr: Address,
    base_program_accounts: strategy_config::base::BaseAccounts,
    eth_program_accounts: strategy_config::ethereum::EthereumAccounts,
) -> Result<strategy_config::base::BaseLibraries, Box<dyn Error>> {
    info!("Setting up all libraries on Base");

    // The strategist will be the processor for simplicity
    let processor = base_strategist_addr;

    let pancake_position_manager = set_up_pancake_position_manager(
        base_client,
        Address::from_str(&base_program_accounts.pancake_input)?,
        Address::from_str(&base_program_accounts.pancake_output)?,
        base_admin_addr,
        processor,
    )
    .await?;

    let cctp_transfer = set_up_cctp_transfer(
        base_client,
        Address::from_str(&base_program_accounts.pancake_input)?,
        Address::from_str(&eth_program_accounts.aave_input)?,
        base_admin_addr,
        processor,
    )
    .await?;

    let standard_bridge_transfer = set_up_standard_bridge_transfer(
        base_client,
        Address::from_str(&base_program_accounts.standard_bridge_input)?,
        Address::from_str(&eth_program_accounts.aave_input)?,
        base_admin_addr,
        processor,
    )
    .await?;

    let forwarder_pancake_output_to_input = set_up_forwarder_pancake_output_to_input(
        base_client,
        Address::from_str(&base_program_accounts.pancake_output)?,
        Address::from_str(&base_program_accounts.pancake_input)?,
        base_admin_addr,
        processor,
    )
    .await?;

    let forwarder_pancake_output_to_cctp_input = set_up_forwarder_pancake_to_cctp(
        base_client,
        Address::from_str(&base_program_accounts.pancake_output)?,
        Address::from_str(&base_program_accounts.cctp_input)?,
        base_admin_addr,
        processor,
    )
    .await?;

    let forwarder_pancake_output_to_standard_bridge_input =
        set_up_forwarder_pancake_to_standard_bridge(
            base_client,
            Address::from_str(&base_program_accounts.pancake_output)?,
            Address::from_str(&base_program_accounts.standard_bridge_input)?,
            base_admin_addr,
            processor,
        )
        .await?;

    let libraries = strategy_config::base::BaseLibraries {
        pancake_position_manager: pancake_position_manager.to_string(),
        cctp_transfer: cctp_transfer.to_string(),
        standard_bridge_transfer: standard_bridge_transfer.to_string(),
        pancake_output_to_input_forwarder: forwarder_pancake_output_to_input.to_string(),
        pancake_output_to_cctp_input_forwarder: forwarder_pancake_output_to_cctp_input.to_string(),
        pancake_output_to_standard_bridge_input_forwarder:
            forwarder_pancake_output_to_standard_bridge_input.to_string(),
    };

    Ok(libraries)
}

async fn set_up_pancake_position_manager(
    base_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Pancake Position Manager on Base");

    let pancake_position_manager_config = PancakeSwapV3PositionManagerConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        positionManager: alloy_primitives_encoder::Address::from_str(
            PANCAKE_POSITION_MANAGER_ON_BASE,
        )?,
        masterChef: alloy_primitives_encoder::Address::from_str(PANCAKE_MASTERCHEF_ON_BASE)?,
        token0: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_BASE)?,
        token1: alloy_primitives_encoder::Address::from_str(USDC_ADDRESS_ON_BASE)?,
        poolFee: 100,
        slippageBps: 1000,       // 10% slippage
        timeout: U256::from(60), // 10 seconds
    };

    let pancake_position_manager_tx = PancakeSwapV3PositionManager::deploy_builder(
        &base_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&pancake_position_manager_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = base_client.execute_tx(pancake_position_manager_tx).await?;
    let pancake_position_manager_addr = response.contract_address.unwrap();

    info!("Pancake Position Manager deployed at: {pancake_position_manager_addr}");

    info!(
        "Deployed Pancake Position Manager contract at address: {:?}",
        pancake_position_manager_addr
    );

    // Approve the vault on input account
    approve_library(base_client, pancake_position_manager_addr, input_account).await?;

    Ok(pancake_position_manager_addr)
}

async fn set_up_cctp_transfer(
    base_client: &EthereumClient,
    input_account: Address,
    mint_recipient: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up CCTP Transfer on Base");

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
        destinationDomain: 0,
        cctpTokenMessenger: alloy_primitives_encoder::Address::from_str(
            CCTP_TOKEN_MESSENGER_ON_BASE,
        )?,
        transferToken: alloy_primitives_encoder::Address::from_str(USDC_ADDRESS_ON_BASE)?,
    };

    let cctp_transfer_tx = CCTPTransfer::deploy_builder(
        &base_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&cctp_transer_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = base_client.execute_tx(cctp_transfer_tx).await?;
    let cctp_transfer_addr = response.contract_address.unwrap();

    info!("CCTP Transfer deployed at: {cctp_transfer_addr}");

    // Approve the vault on input account
    approve_library(base_client, cctp_transfer_addr, input_account).await?;

    Ok(cctp_transfer_addr)
}

async fn set_up_standard_bridge_transfer(
    base_client: &EthereumClient,
    input_account: Address,
    recipient: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Standard Bridge Transfer on Base");

    let standard_bridge_transfer_config = StandardBridgeTransferConfig {
        amount: U256::ZERO,
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        recipient: alloy_primitives_encoder::Address::from_str(recipient.to_string().as_str())?,
        standardBridge: alloy_primitives_encoder::Address::from_str(L2_STANDARD_BRIDGE_ADDRESS)?,
        token: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_BASE)?,
        remoteToken: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_ETHEREUM)?,
        minGasLimit: 200000,
        extraData: alloy_primitives_encoder::Bytes::new(),
    };

    let standard_bridge_transfer_tx = StandardBridgeTransfer::deploy_builder(
        &base_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&standard_bridge_transfer_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = base_client.execute_tx(standard_bridge_transfer_tx).await?;
    let standard_bridge_transfer_address = response.contract_address.unwrap();

    info!("Standard Bridge Transfer deployed at: {standard_bridge_transfer_address}");

    // Approve the vault on input account
    approve_library(base_client, standard_bridge_transfer_address, input_account).await?;

    Ok(standard_bridge_transfer_address)
}

async fn set_up_forwarder_pancake_output_to_input(
    base_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Forwarder Pancake Output to Input on Ethereum");

    let forwarder_pancake_input_to_output_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        // Strategist will update this to forward the right amount
        forwardingConfigs: vec![
            ForwardingConfig {
                tokenAddress: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_BASE)?,
                maxAmount: 0,
            },
            ForwardingConfig {
                tokenAddress: alloy_primitives_encoder::Address::from_str(USDC_ADDRESS_ON_BASE)?,
                maxAmount: 0,
            },
        ],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };

    let forwarder_pancake_input_to_output_tx = Forwarder::deploy_builder(
        &base_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&forwarder_pancake_input_to_output_config)
            .into(),
    )
    .into_transaction_request()
    .from(admin);

    let response: alloy::rpc::types::TransactionReceipt = base_client
        .execute_tx(forwarder_pancake_input_to_output_tx)
        .await?;
    let forwarder_pancake_input_to_output_address = response.contract_address.unwrap();

    info!("Forwarder Pancake input to output deployed at: {forwarder_pancake_input_to_output_address}");

    // Approve the vault on input account
    approve_library(
        base_client,
        forwarder_pancake_input_to_output_address,
        input_account,
    )
    .await?;

    Ok(forwarder_pancake_input_to_output_address)
}

async fn set_up_forwarder_pancake_to_standard_bridge(
    base_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Forwarder Pancake to Standard Bridge on Ethereum");

    let forwarder_pancake_to_standard_bridge_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        // Strategist will update this to forward the right amount
        forwardingConfigs: vec![ForwardingConfig {
            tokenAddress: alloy_primitives_encoder::Address::from_str(WETH_ADDRESS_ON_BASE)?,
            maxAmount: 0,
        }],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };

    let forwarder_pancake_to_standard_bridge_tx = Forwarder::deploy_builder(
        &base_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&forwarder_pancake_to_standard_bridge_config)
            .into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = base_client
        .execute_tx(forwarder_pancake_to_standard_bridge_tx)
        .await?;
    let forwarder_pancake_to_standard_bridge_address = response.contract_address.unwrap();

    info!("Forwarder Pancake to Standard Bridge deployed at: {forwarder_pancake_to_standard_bridge_address}");

    // Approve the vault on input account
    approve_library(
        base_client,
        forwarder_pancake_to_standard_bridge_address,
        input_account,
    )
    .await?;

    Ok(forwarder_pancake_to_standard_bridge_address)
}

async fn set_up_forwarder_pancake_to_cctp(
    base_client: &EthereumClient,
    input_account: Address,
    output_account: Address,
    admin: Address,
    processor: Address,
) -> Result<Address, Box<dyn Error>> {
    info!("Setting up Forwarder Pancake to CCTP on Ethereum");

    let forwarder_pancake_to_cctp_config = ForwarderConfig {
        inputAccount: alloy_primitives_encoder::Address::from_str(
            input_account.to_string().as_str(),
        )?,
        outputAccount: alloy_primitives_encoder::Address::from_str(
            output_account.to_string().as_str(),
        )?,
        // Strategist will update this to forward the right amount
        forwardingConfigs: vec![ForwardingConfig {
            tokenAddress: alloy_primitives_encoder::Address::from_str(USDC_ADDRESS_ON_BASE)?,
            maxAmount: 0,
        }],
        intervalType: IntervalType::TIME,
        minInterval: 0,
    };

    let forwarder_pancake_to_cctp_tx = Forwarder::deploy_builder(
        &base_client.get_request_provider().await?,
        admin,
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&forwarder_pancake_to_cctp_config).into(),
    )
    .into_transaction_request()
    .from(admin);

    let response = base_client.execute_tx(forwarder_pancake_to_cctp_tx).await?;
    let forwarder_pancake_to_cctp_address = response.contract_address.unwrap();

    info!("Forwarder Pancake to CCTP deployed at: {forwarder_pancake_to_cctp_address}");

    // Approve the vault on input account
    approve_library(
        base_client,
        forwarder_pancake_to_cctp_address,
        input_account,
    )
    .await?;

    Ok(forwarder_pancake_to_cctp_address)
}
