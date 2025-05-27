use std::{error::Error, str::FromStr};

use alloy::{
    network::{EthereumWallet, TransactionBuilder},
    primitives::{Address, Bytes, U256},
    providers::Provider,
    sol_types::SolValue,
};

use valence_domain_clients::{
    clients::ethereum::EthereumClient, evm::request_provider_client::RequestProviderClient,
};
use valence_e2e::utils::solidity_contracts::{
    BaseAccount, ERC1967Proxy, MockERC20,
    OneWayVault::{self, FeeDistributionConfig, OneWayVaultConfig},
    ERC20,
};

//const ETH_ENDPOINT: &str = "https://eth-mainnet.public.blastapi.io";
//const MNEMONIC: &str = "test test test test test test test test test test test junk";
//const ETH_ENDPOINT: &str = "http://127.0.0.1:8545";
const ETH_ENDPOINT: &str = "https://eth-sepolia.public.blastapi.io";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let eth_client = EthereumClient::new(ETH_ENDPOINT, MNEMONIC, None)?;
    let wallet = EthereumWallet::from(eth_client.signer());
    let rp = eth_client.get_request_provider().await?;

    let my_address = eth_client.signer().address();

    let nonce = rp.get_transaction_count(my_address).await?;
    let deposit_account_tx = BaseAccount::deploy_builder(&rp, my_address, vec![])
        .into_transaction_request()
        .with_nonce(nonce);

    let tx_request = rp
        .fill(deposit_account_tx)
        .await?
        .as_builder()
        .unwrap()
        .clone();
    let tx_envelope = tx_request.build(&wallet).await?;

    let deposit_account_address = rp
        .send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?
        .contract_address
        .unwrap();

    /*let deposit_account_address =
    Address::from_str("0xe7e31c0E1F94d94b4a113592e7a9Cff8bDCA84Be").unwrap();*/
    println!("Deposit account deployed at: {deposit_account_address}");

    let fee_distribution_config = FeeDistributionConfig {
        strategistAccount: deposit_account_address,
        platformAccount: deposit_account_address,
        strategistRatioBps: 1000,
    };

    let one_way_vault_config = OneWayVaultConfig {
        depositAccount: deposit_account_address,
        strategist: deposit_account_address,
        depositFeeBps: 0,
        withdrawRateBps: 0,
        depositCap: U256::ZERO,
        feeDistribution: fee_distribution_config,
    };

    let nonce = rp.get_transaction_count(my_address).await?;
    let implementation_tx = OneWayVault::deploy_builder(&rp)
        .into_transaction_request()
        .with_nonce(nonce);

    let tx_request = rp
        .fill(implementation_tx)
        .await?
        .as_builder()
        .unwrap()
        .clone();
    let tx_envelope = tx_request.build(&wallet).await?;

    let implementation_address = rp
        .send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?
        .contract_address
        .unwrap();

    let nonce = rp.get_transaction_count(my_address).await?;
    let proxy_tx = ERC1967Proxy::deploy_builder(&rp, implementation_address, Bytes::new())
        .into_transaction_request()
        .with_nonce(nonce);

    let tx_request = rp.fill(proxy_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;
    let proxy_address = rp
        .send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?
        .contract_address
        .unwrap();

    println!("Vault deployed at: {proxy_address}");

    //let wbtc_address = Address::from_str("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599").unwrap();

    let nonce = rp.get_transaction_count(my_address).await?;

    let token_1_tx = MockERC20::deploy_builder(&rp, "WBTC".to_string(), "WBTC".to_string(), 8)
        .into_transaction_request()
        .with_nonce(nonce);

    let tx_request = rp.fill(token_1_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;

    let wbtc_address = rp
        .send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?
        .contract_address
        .unwrap();

    // Mint some
    let wbtc = MockERC20::new(wbtc_address, &rp);
    let mint_amount = U256::from(1000000);
    let nonce = rp.get_transaction_count(my_address).await?;
    let mint_tx = wbtc
        .mint(my_address, mint_amount)
        .into_transaction_request()
        .with_nonce(nonce);
    let tx_request = rp.fill(mint_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;
    rp.send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?;
    println!("Minted {} WBTC to {}", mint_amount, my_address);

    // Check my balance
    let balance_call = wbtc.balanceOf(my_address).call().await?._0;
    println!("My WBTC balance: {balance_call}");

    let vault = OneWayVault::new(proxy_address, &rp);

    let nonce = rp.get_transaction_count(my_address).await?;
    let initialize_tx = vault
        .initialize(
            my_address,
            one_way_vault_config.abi_encode().into(),
            wbtc_address,
            "Valence One Way Vault".to_string(), // vault token name
            "vTEST".to_string(),                 // vault token symbol
            U256::from(1e8),                     // match deposit token precision
        )
        .into_transaction_request()
        .with_nonce(nonce);

    let tx_request = rp.fill(initialize_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;
    rp.send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?;

    println!("Vault initialized with WBTC at address: {wbtc_address}");

    // Let's deposit and withdraw some WBTC into the vault
    //let wbtc = ERC20::new(wbtc_address, &rp);
    let deposit_amount = U256::from(100);

    let nonce = rp.get_transaction_count(my_address).await?;
    let approve_tx = wbtc
        .approve(proxy_address, deposit_amount)
        .into_transaction_request()
        .with_nonce(nonce)
        .from(my_address);
    let tx_request = rp.fill(approve_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;
    rp.send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?;

    println!("Approved WBTC for vault deposit");

    let nonce = rp.get_transaction_count(my_address).await?;
    let deposit_tx = vault
        .deposit(deposit_amount, my_address)
        .into_transaction_request()
        .with_nonce(nonce)
        .from(my_address);

    let tx_request = rp.fill(deposit_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;
    rp.send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?;

    // Get how many shares we received
    let balance_shares = vault.balanceOf(my_address).call().await?._0;
    println!("Shares balance in vault: {:?} shares", balance_shares);

    // Now let's withdraw the same amount
    let nonce = rp.get_transaction_count(my_address).await?;
    let withdraw_tx = vault
        .redeem_0(
            balance_shares,
            "neutron14mlpd48k5vkeset4x7f78myz3m47jcax3ysjkp".to_string(),
            my_address,
        )
        .into_transaction_request()
        .with_nonce(nonce)
        .from(my_address);
    let tx_request = rp.fill(withdraw_tx).await?.as_builder().unwrap().clone();
    let tx_envelope = tx_request.build(&wallet).await?;
    rp.send_tx_envelope(tx_envelope)
        .await?
        .get_receipt()
        .await?;
    println!("Withdrawn shares from vault");

    // Check the withdraw request for ID 0
    let withdraw_request = vault.withdrawRequests(0).call().await?;
    println!("Withdraw request: {:?}", withdraw_request);

    Ok(())
}
