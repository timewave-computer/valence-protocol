use std::{error::Error, path::Path, str::FromStr};

use alloy::primitives::{Address, U256};
use base::set_up_base_accounts;
use ethereum::set_up_eth_accounts;
use log::info;
use strategist::{
    strategy::Strategy,
    strategy_config::{
        base::{BaseDenoms, BaseStrategyConfig},
        ethereum::{EthereumContracts, EthereumDenoms, EthereumParameters, EthereumStrategyConfig},
        StrategyConfig,
    },
};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{
        anvil::AnvilImpersonationClient, base_client::EvmBaseClient,
        request_provider_client::RequestProviderClient,
    },
};
use valence_e2e::utils::{
    ethereum::set_up_anvil_container,
    mocks::{
        cctp_relayer_evm_evm::MockCctpRelayerEvmEvm,
        standard_bridge_relayer::MockStandardBridgeRelayer,
    },
    solidity_contracts::{BaseAccount, ValenceVault, ERC20},
    worker::{ValenceWorker, ValenceWorkerTomlSerde},
};

const ETH_FORK_URL: &str = "https://eth-mainnet.public.blastapi.io";
const ETH_ANVIL_PORT: &str = "1337";
const BASE_FORK_URL: &str = "https://mainnet.base.org";
const BASE_ANVIL_PORT: &str = "1338";
const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
pub const WETH_ADDRESS_ON_ETHEREUM: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";
pub const WETH_ADDRESS_ON_BASE: &str = "0x4200000000000000000000000000000000000006";
pub const USDC_ADDRESS_ON_ETHEREUM: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const USDC_ADDRESS_ON_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
pub const CAKE_ADDRESS_ON_BASE: &str = "0x3055913c90Fcc1A6CE9a358911721eEb942013A1";
pub const CCTP_TOKEN_MESSENGER_ON_ETHEREUM: &str = "0xBd3fa81B58Ba92a82136038B25aDec7066af3155";
pub const CCTP_TOKEN_MESSENGER_ON_BASE: &str = "0x1682Ae6375C4E4A97e4B583BC394c861A46D8962";
pub const AAVE_POOL_ADDRESS: &str = "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2";
pub const L1_STANDARD_BRIDGE_ADDRESS: &str = "0x3154Cf16ccdb4C6d922629664174b904d80F2C35";
pub const L2_STANDARD_BRIDGE_ADDRESS: &str = "0x4200000000000000000000000000000000000010";
pub const PANCAKE_POSITION_MANAGER_ON_BASE: &str = "0x46A15B0b27311cedF172AB29E4f4766fbE7F4364";
pub const PANCAKE_MASTERCHEF_ON_BASE: &str = "0xC6A2Db661D5a5690172d8eB0a7DEA2d3008665A3";

mod base;
mod ethereum;
mod strategist;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up the Anvil container for Ethereum
    set_up_anvil_container("anvil_ethereum", ETH_ANVIL_PORT, Some(ETH_FORK_URL)).await?;

    // Set up the Anvil container for Base
    set_up_anvil_container("anvil_base", BASE_ANVIL_PORT, Some(BASE_FORK_URL)).await?;

    let endpoint_eth = format!("http://127.0.0.1:{}", ETH_ANVIL_PORT);
    let endpoint_base = format!("http://127.0.0.1:{}", BASE_ANVIL_PORT);

    // Create an Ethereum client
    let eth_client = EthereumClient::new(&endpoint_eth, TEST_MNEMONIC)?;

    // Create a Base client
    let base_client = EthereumClient::new(&endpoint_base, TEST_MNEMONIC)?;

    // Get an admin account for Ethereum
    let accounts_eth = eth_client.get_provider_accounts().await?;
    let strategist_acc = accounts_eth[7];
    let eth_admin_addr = accounts_eth[7]; // Strategist account is admin because it will update configs

    // Create all the acounts needed for Ethereum
    let ethereum_accounts = set_up_eth_accounts(&eth_client, eth_admin_addr).await?;

    // Get an admin account for Base
    let accounts_base = base_client.get_provider_accounts().await?;
    let base_admin_addr = accounts_base[7]; // Strategist account is admin because it will update configs

    // Create all the accounts needed for Base
    let base_accounts = set_up_base_accounts(&base_client, base_admin_addr).await?;

    // Set up ethereum libraries
    let ethereum_libraries = ethereum::set_up_eth_libraries(
        &eth_client,
        eth_admin_addr, // admin
        strategist_acc, // strategist
        strategist_acc, // platform fee receiver
        ethereum_accounts.clone(),
        base_accounts.clone(),
    )
    .await?;

    info!(
        "Ethereum libraries set up successfully: {:?}",
        ethereum_libraries
    );

    // Set up base libraries
    let base_libraries = base::set_up_base_libraries(
        &base_client,
        base_admin_addr, // admin
        strategist_acc,  // strategist
        base_accounts.clone(),
        ethereum_accounts.clone(),
    )
    .await?;

    info!("Base libraries set up successfully: {:?}", base_libraries);

    info!("Setting up mock relayers for Standard Bridge and CCTP...");
    let weth_whale_on_eth = "0x57757E3D981446D585Af0D9Ae4d7DF6D64647806";
    let weth_whale_on_base = "0xbcb375D0599896Fedfa8D8f82cF6ede0754BF1b6";
    let usdc_whale_on_eth = "0x28C6c06298d514Db089934071355E5743bf21d60";
    let usdc_whale_on_base = "0x3304E22DDaa22bCdC5fCa2269b418046aE7b566A";

    let mock_standard_bridge_relayer_addr =
        Address::from_str("0x976EA74026E726554dB657fA54763abd0C3a0aa9")?;
    let mock_cctp_relayer_addr = Address::from_str("0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc")?;

    let standard_bridge_eth = Address::from_str(L1_STANDARD_BRIDGE_ADDRESS)?;
    let cctp_token_messenger_eth = Address::from_str(CCTP_TOKEN_MESSENGER_ON_ETHEREUM)?;
    let standard_bridge_base = Address::from_str(L2_STANDARD_BRIDGE_ADDRESS)?;
    let cctp_token_messenger_base = Address::from_str(CCTP_TOKEN_MESSENGER_ON_BASE)?;

    // Fund them with enough tokens
    let weth_amount_to_fund = 100e18;
    let usdc_amount_to_fund = 1000000e6;

    let usdc_address_eth = Address::from_str(USDC_ADDRESS_ON_ETHEREUM)?;
    let weth_address_eth = Address::from_str(WETH_ADDRESS_ON_ETHEREUM)?;
    let usdc_address_base = Address::from_str(USDC_ADDRESS_ON_BASE)?;
    let weth_address_base = Address::from_str(WETH_ADDRESS_ON_BASE)?;

    let usdc_on_eth = ERC20::new(usdc_address_eth, eth_client.get_request_provider().await?);
    let send_tx = usdc_on_eth
        .transfer(mock_cctp_relayer_addr, U256::from(usdc_amount_to_fund))
        .into_transaction_request();
    eth_client.execute_tx_as(usdc_whale_on_eth, send_tx).await?;
    let usdc_on_base = ERC20::new(usdc_address_base, base_client.get_request_provider().await?);
    let send_tx = usdc_on_base
        .transfer(mock_cctp_relayer_addr, U256::from(usdc_amount_to_fund))
        .into_transaction_request();
    base_client
        .execute_tx_as(usdc_whale_on_base, send_tx)
        .await?;

    let weth_on_eth = ERC20::new(weth_address_eth, eth_client.get_request_provider().await?);
    let send_tx = weth_on_eth
        .transfer(
            mock_standard_bridge_relayer_addr,
            U256::from(weth_amount_to_fund),
        )
        .into_transaction_request();
    eth_client.execute_tx_as(weth_whale_on_eth, send_tx).await?;
    let weth_on_base = ERC20::new(weth_address_base, base_client.get_request_provider().await?);
    let send_tx = weth_on_base
        .transfer(
            mock_standard_bridge_relayer_addr,
            U256::from(weth_amount_to_fund),
        )
        .into_transaction_request();
    base_client
        .execute_tx_as(weth_whale_on_base, send_tx)
        .await?;
    info!("Mock relayers funded successfully");

    info!("Starting relayers...");
    let mock_cctp_relayer = MockCctpRelayerEvmEvm::new(
        endpoint_eth.clone(),
        endpoint_base.clone(),
        cctp_token_messenger_eth,
        usdc_address_eth,
        cctp_token_messenger_base,
        usdc_address_base,
    )
    .await?;
    mock_cctp_relayer.start();

    let mock_standard_bridge_relayer = MockStandardBridgeRelayer::new(
        endpoint_eth.clone(),
        endpoint_base.clone(),
        standard_bridge_eth,
        weth_address_eth,
        standard_bridge_base,
        weth_address_base,
    )
    .await?;
    mock_standard_bridge_relayer.start();
    info!("Relayers started successfully");

    info!("Build strategy config...");
    let strategy_config = StrategyConfig {
        ethereum: EthereumStrategyConfig {
            rpc_url: endpoint_eth.clone(),
            mnemonic: TEST_MNEMONIC.to_string(),
            denoms: EthereumDenoms {
                weth: WETH_ADDRESS_ON_ETHEREUM.to_string(),
                usdc: USDC_ADDRESS_ON_ETHEREUM.to_string(),
            },
            accounts: ethereum_accounts.clone(),
            libraries: ethereum_libraries.clone(),
            parameters: EthereumParameters {
                min_aave_health_factor: "1.2".to_string(),
            },
            contracts: EthereumContracts {
                aave_pool: AAVE_POOL_ADDRESS.to_string(),
            },
        },
        base: BaseStrategyConfig {
            rpc_url: endpoint_base.clone(),
            mnemonic: TEST_MNEMONIC.to_string(),
            denoms: BaseDenoms {
                weth: WETH_ADDRESS_ON_BASE.to_string(),
                usdc: USDC_ADDRESS_ON_BASE.to_string(),
                cake: CAKE_ADDRESS_ON_BASE.to_string(),
            },
            accounts: base_accounts.clone(),
            libraries: base_libraries.clone(),
        },
    };

    let temp_path =
        Path::new("./e2e/examples/eth_to_base_vault_pancake/strategist/example_strategy.toml");
    strategy_config.to_file(temp_path)?;
    let strategy = Strategy::from_file(temp_path).await?;

    let mut ethereum_users = vec![];
    for account in accounts_eth.iter().take(4).skip(1) {
        ethereum_users.push(*account);
        let send_tx = weth_on_eth
            .transfer(
                *account,
                U256::from(1e18), // 1 WETH
            )
            .into_transaction_request();
        eth_client.execute_tx_as(weth_whale_on_eth, send_tx).await?;
    }

    let vault_address = Address::from_str(&ethereum_libraries.vault).unwrap();
    let rp = eth_client.get_request_provider().await?;
    let valence_vault = ValenceVault::new(vault_address, &rp);

    {
        info!("\n======================== EPOCH 0 ========================\n");
        info!("User 1 deposits 0.5 WETH into the vault ...");
        let deposit_amount = U256::from(5e17);
        let approval = weth_on_eth
            .approve(vault_address, deposit_amount)
            .into_transaction_request();
        eth_client
            .execute_tx_as(&ethereum_users[0].to_string(), approval)
            .await?;
        let deposit = valence_vault
            .deposit(deposit_amount, ethereum_users[0])
            .into_transaction_request();
        eth_client
            .execute_tx_as(&ethereum_users[0].to_string(), deposit)
            .await?;
        let balance_shares_user1 = valence_vault.balanceOf(ethereum_users[0]).call().await?._0;
        info!("User 1 balance in vault: {:?} shares", balance_shares_user1);

        info!("User 2 deposits 0.3 WETH into the vault ...");
        let deposit_amount = U256::from(3e17);
        let approval = weth_on_eth
            .approve(vault_address, deposit_amount)
            .into_transaction_request();
        eth_client
            .execute_tx_as(&ethereum_users[1].to_string(), approval)
            .await?;
        let deposit = valence_vault
            .deposit(deposit_amount, ethereum_users[1])
            .into_transaction_request();
        eth_client
            .execute_tx_as(&ethereum_users[1].to_string(), deposit)
            .await?;
        let balance_shares_user2 = valence_vault.balanceOf(ethereum_users[1]).call().await?._0;
        info!("User 2 balance in vault: {:?} shares", balance_shares_user2);

        info!("User 3 deposits 0.2 WETH into the vault ...");
        let deposit_amount = U256::from(2e17);
        let approval = weth_on_eth
            .approve(vault_address, deposit_amount)
            .into_transaction_request();
        eth_client
            .execute_tx_as(&ethereum_users[2].to_string(), approval)
            .await?;
        let deposit = valence_vault
            .deposit(deposit_amount, ethereum_users[2])
            .into_transaction_request();
        eth_client
            .execute_tx_as(&ethereum_users[2].to_string(), deposit)
            .await?;
        let balance_shares_user3 = valence_vault.balanceOf(ethereum_users[2]).call().await?._0;
        info!("User 3 balance in vault: {:?} shares", balance_shares_user3);
    }

    strategy.start();

    // Sleep for 2 minutes
    tokio::time::sleep(tokio::time::Duration::from_secs(120)).await;

    Ok(())
}

// Helper function to approve a library from a Base Account
pub async fn approve_library(
    client: &EthereumClient,
    library: Address,
    account: Address,
) -> Result<(), Box<dyn Error>> {
    let rp = client.get_request_provider().await?;

    // Approve the library on the account
    info!("Approving library {} on account {}...", library, account);
    let base_account = BaseAccount::new(account, &rp);

    client
        .execute_tx(
            base_account
                .approveLibrary(library)
                .into_transaction_request(),
        )
        .await?;

    Ok(())
}
