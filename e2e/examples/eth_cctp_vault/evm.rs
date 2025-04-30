use std::error::Error;

use std::str::FromStr;

use alloy::{
    hex::FromHex,
    primitives::{Address, U256},
};
use log::info;
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_encoder_utils::libraries::cctp_transfer::solidity_types::CCTPTransferConfig;

use crate::{async_run, strategist::strategy_config};
use valence_e2e::utils::{
    solidity_contracts::{
        CCTPTransfer, MockTokenMessenger,
        ValenceVault::{FeeConfig, FeeDistributionConfig, VaultConfig},
    },
    vault::setup_valence_vault,
};

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
pub fn setup_eth_libraries(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
    eth_strategist_addr: Address,
    eth_program_accounts: strategy_config::ethereum::EthereumAccounts,
    cctp_messenger_addr: Address,
    usdc_token_addr: Address,
    noble_inbound_ica_addr: String,
    eth_hyperlane_mailbox_addr: String,
    ntrn_authorizations_addr: String,
    eth_accounts: &[Address],
) -> Result<strategy_config::ethereum::EthereumLibraries, Box<dyn Error>> {
    info!("Setting up CCTP Transfer on Ethereum");
    let cctp_forwarder_addr = setup_cctp_transfer(
        rt,
        eth_client,
        noble_inbound_ica_addr,
        eth_program_accounts.deposit.to_string(),
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

    let fee_config = FeeConfig {
        depositFeeBps: 0,          // No deposit fee
        platformFeeBps: 10_000,    // 0.1% yearly platform fee
        performanceFeeBps: 10_000, // 0.1% performance fee
        solverCompletionFee: 0,    // No solver completion fee
    };

    let fee_distribution = FeeDistributionConfig {
        strategistAccount: eth_accounts[0], // Strategist fee recipient
        platformAccount: eth_accounts[1],   // Platform fee recipient
        strategistRatioBps: 10_000,         // 0.1% to strategist
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
        usdc_token_addr,
        vault_config,
    )?;

    let libraries = strategy_config::ethereum::EthereumLibraries {
        cctp_forwarder: cctp_forwarder_addr.to_string(),
        lite_processor: lite_processor_address.to_string(),
        valence_vault: vault_address.to_string(),
    };

    Ok(libraries)
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
    input_account: String,
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
        amount: U256::ZERO,
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
        processor,
        alloy_sol_types_encoder::SolValue::abi_encode(&cctp_transer_cfg).into(),
    )
    .into_transaction_request()
    .from(admin);

    let cctp_rx = async_run!(rt, eth_client.execute_tx(cctp_tx).await.unwrap());

    let cctp_address = cctp_rx.contract_address.unwrap();
    info!("CCTP Transfer deployed at: {cctp_address}");

    // approve the CCTP forwarder on deposit account
    valence_e2e::utils::ethereum::valence_account::approve_library(
        rt,
        eth_client,
        Address::from_str(&input_account).unwrap(),
        cctp_address,
    );

    Ok(cctp_address)
}
