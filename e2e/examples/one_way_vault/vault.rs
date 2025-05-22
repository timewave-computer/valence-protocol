use std::error::Error;

use alloy::{primitives::{Bytes, U256}, sol_types::SolValue};
use valence_domain_clients::{
    clients::ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    ethereum::{set_up_anvil_container, DEFAULT_ANVIL_PORT},
    solidity_contracts::{
        BaseAccount, ERC1967Proxy, MockERC20, OneWayVault::{self, FeeDistributionConfig, OneWayVaultConfig}
    },
    DEFAULT_ANVIL_RPC_ENDPOINT,
};

const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up the Anvil container for Ethereum
    set_up_anvil_container("anvil_ethereum", DEFAULT_ANVIL_PORT, None).await?;

    let eth_client = EthereumClient::new(DEFAULT_ANVIL_RPC_ENDPOINT, TEST_MNEMONIC, None)?;
    let accounts = eth_client.get_provider_accounts().await?;

    let token_1_tx = MockERC20::deploy_builder(
        &eth_client.get_request_provider().await?,
        "LBTC".to_string(),
        "LBTC".to_string(),
        8,
    )
    .into_transaction_request()
    .from(accounts[0]);
    let lbtc_address = eth_client.execute_tx(token_1_tx).await?.contract_address.unwrap();
    println!("Deployed LBTC contract at address: {:?}", lbtc_address);

    let deposit_account_tx = BaseAccount::deploy_builder(
        &eth_client.get_request_provider().await?,
        accounts[0],
        vec![],
    )
    .into_transaction_request();

    let deposit_account_address = eth_client
        .execute_tx(deposit_account_tx.clone())
        .await?
        .contract_address
        .unwrap();

    let fee_distribution_config = FeeDistributionConfig {
        strategistAccount: deposit_account_address,
        platformAccount: deposit_account_address,
        strategistRatioBps: 1000,
    };

    let one_way_vault_config = OneWayVaultConfig {
        depositAccount: deposit_account_address,
        strategist: deposit_account_address,
        depositFeeBps: 0,
        depositCap: U256::ZERO,
        feeDistribution: fee_distribution_config,
    };

    let implementation_tx = OneWayVault::deploy_builder(&eth_client.get_request_provider().await?)
        .into_transaction_request();

    let implementation_address = eth_client
        .execute_tx(implementation_tx)
        .await?
        .contract_address
        .unwrap();

    let proxy_tx = ERC1967Proxy::deploy_builder(
        &eth_client.get_request_provider().await?,
        implementation_address,
        Bytes::new(),
    )
    .into_transaction_request();

    let proxy_address = eth_client
        .execute_tx(proxy_tx)
        .await?
        .contract_address
        .unwrap();

    println!("Vault deployed at: {proxy_address}");

    let rp = eth_client.get_request_provider().await?;
    let vault = OneWayVault::new(proxy_address, &rp);

    let initialize_tx = vault
        .initialize(
            accounts[0],
            one_way_vault_config.abi_encode().into(),        
            lbtc_address,
            "Valence One Way Vault".to_string(),                       // vault token name
            "vTEST".to_string(),                                     // vault token symbol
            U256::from(1e8),                                        // match deposit token precision
        )
        .into_transaction_request();

    eth_client.execute_tx(initialize_tx).await?;

    Ok(())
}
