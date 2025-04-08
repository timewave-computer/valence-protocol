use std::error::Error;

use std::str::FromStr;

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
use valence_encoder_utils::libraries::cctp_transfer::solidity_types::CCTPTransferConfig;

use crate::async_run;
use valence_e2e::utils::{
    ethereum::mock_erc20,
    solidity_contracts::{
        CCTPTransfer, ERC1967Proxy, MockERC20, MockTokenMessenger,
        ValenceVault::{self, FeeConfig, FeeDistributionConfig, VaultConfig},
    },
    vault,
};

pub fn mine_blocks(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    blocks: usize,
    interval: usize,
) {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        alloy::providers::ext::AnvilApi::anvil_mine(
            &eth_rp,
            Some(U256::from(blocks)),
            Some(U256::from(interval)),
        )
        .await
        .unwrap();
    });
}

#[derive(Clone, Debug)]
pub struct EthereumProgramLibraries {
    pub cctp_forwarder: Address,
    pub _lite_processor: Address,
    pub valence_vault: Address,
}

#[derive(Clone, Debug)]
pub struct EthereumProgramAccounts {
    pub deposit: Address,
    pub withdraw: Address,
}

impl EthereumProgramAccounts {
    pub async fn log_balances(
        &self,
        eth_client: &EthereumClient,
        vault_addr: &Address,
        vault_deposit_token: &Address,
    ) {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
        let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

        let deposit_usdc_bal = eth_client
            .query(usdc_token.balanceOf(self.deposit))
            .await
            .unwrap()
            ._0;
        let deposit_vault_bal = eth_client
            .query(valence_vault.balanceOf(self.deposit))
            .await
            .unwrap()
            ._0;
        let deposit_usdc_entry = format!("{deposit_usdc_bal}USDC");
        let deposit_vault_entry = format!("{deposit_vault_bal}VAULT");

        let withdraw_usdc_bal = eth_client
            .query(usdc_token.balanceOf(self.withdraw))
            .await
            .unwrap()
            ._0;
        let withdraw_vault_bal = eth_client
            .query(valence_vault.balanceOf(self.withdraw))
            .await
            .unwrap()
            ._0;
        let withdraw_usdc_entry = format!("{withdraw_usdc_bal}USDC");
        let withdraw_vault_entry = format!("{withdraw_vault_bal}VAULT");

        info!("\n[STRATEGIST] ETHEREUM PROGRAM ACCOUNTS LOG");
        info!("\tDEPOSIT: {deposit_usdc_entry} {deposit_vault_entry}");
        info!("\tWITHDRAW: {withdraw_usdc_entry} {withdraw_vault_entry}")
    }
}

#[derive(Clone, Debug)]
pub struct EthereumUsers {
    pub users: Vec<Address>,
    pub erc20: Address,
    pub vault: Address,
}

impl EthereumUsers {
    pub fn new(erc20: Address, vault: Address) -> Self {
        Self {
            users: vec![],
            erc20,
            vault,
        }
    }

    pub fn add_user(
        &mut self,
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        user: Address,
    ) {
        info!("Adding new user {user}");
        self.users.push(user);
        info!("Approving erc20 spend for vault on behalf of user");
        mock_erc20::approve(rt, eth_client, self.erc20, user, self.vault, U256::MAX);
    }

    pub fn fund_user(
        &self,
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        user: usize,
        amount: U256,
    ) {
        mock_erc20::mint(rt, eth_client, self.erc20, self.users[user], amount);
    }

    pub fn get_user_shares(
        &self,
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        user: usize,
    ) -> U256 {
        let user_shares_balance =
            vault::query_vault_balance_of(self.vault, rt, eth_client, self.users[user]);
        user_shares_balance._0
    }

    pub fn get_user_usdc(
        &self,
        rt: &tokio::runtime::Runtime,
        eth_client: &EthereumClient,
        user: usize,
    ) -> U256 {
        mock_erc20::query_balance(rt, eth_client, self.erc20, self.users[user])
    }

    pub async fn log_balances(
        &self,
        eth_client: &EthereumClient,
        vault_addr: &Address,
        vault_deposit_token: &Address,
    ) {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
        let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

        info!("\n[STRATEGIST] ETHEREUM ACCOUNTS LOG");
        for (i, user) in self.users.iter().enumerate() {
            let usdc_bal = eth_client
                .query(usdc_token.balanceOf(*user))
                .await
                .unwrap()
                ._0;
            let vault_bal = eth_client
                .query(valence_vault.balanceOf(*user))
                .await
                .unwrap()
                ._0;
            let usdc_entry = format!("{usdc_bal}USDC");
            let vault_entry = format!("{vault_bal}VAULT");
            info!("\tUSER_{i}: {usdc_entry} {vault_entry}");
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn log_eth_balances(
    eth_client: &EthereumClient,
    rt: &tokio::runtime::Runtime,
    vault_addr: &Address,
    vault_deposit_token: &Address,
    eth_program_accounts: &EthereumProgramAccounts,
    eth_users: &EthereumUsers,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
        let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

        let (
            user1_usdc_bal,
            user2_usdc_bal,
            user1_vault_bal,
            user2_vault_bal,
            withdraw_acc_usdc_bal,
            deposit_acc_usdc_bal,
            vault_total_supply,
        ) = tokio::join!(
            eth_client.query(usdc_token.balanceOf(eth_users.users[0])),
            eth_client.query(usdc_token.balanceOf(eth_users.users[1])),
            eth_client.query(valence_vault.balanceOf(eth_users.users[0])),
            eth_client.query(valence_vault.balanceOf(eth_users.users[1])),
            eth_client.query(usdc_token.balanceOf(eth_program_accounts.withdraw)),
            eth_client.query(usdc_token.balanceOf(eth_program_accounts.deposit)),
            eth_client.query(valence_vault.totalSupply()),
        );

        let user1_usdc_bal = user1_usdc_bal.unwrap()._0;
        let user2_usdc_bal = user2_usdc_bal.unwrap()._0;
        let user1_vault_bal = user1_vault_bal.unwrap()._0;
        let user2_vault_bal = user2_vault_bal.unwrap()._0;
        let withdraw_acc_usdc_bal = withdraw_acc_usdc_bal.unwrap()._0;
        let deposit_acc_usdc_bal = deposit_acc_usdc_bal.unwrap()._0;
        let vault_total_supply = vault_total_supply.unwrap()._0;

        info!("USER1 SHARES\t\t: {user1_vault_bal}");
        info!("USER1 USDC\t\t: {user1_usdc_bal}");
        info!("USER2 SHARES\t\t: {user2_vault_bal}");
        info!("USER2 USDC\t\t: {user2_usdc_bal}");
        info!("WITHDRAW ACC USDC\t: {withdraw_acc_usdc_bal}");
        info!("DEPOSIT ACC USDC\t: {deposit_acc_usdc_bal}");
        info!("VAULT TOTAL SUPPLY\t: {vault_total_supply}");
    });

    Ok(())
}

pub fn setup_eth_accounts(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
) -> Result<EthereumProgramAccounts, Box<dyn Error>> {
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

    let accounts = EthereumProgramAccounts {
        deposit: deposit_acc_addr,
        withdraw: withdraw_acc_addr,
    };

    Ok(accounts)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_eth_libraries(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_admin_addr: Address,
    eth_strategist_addr: Address,
    eth_program_accounts: EthereumProgramAccounts,
    cctp_messenger_addr: Address,
    usdc_token_addr: Address,
    noble_inbound_ica_addr: String,
    eth_hyperlane_mailbox_addr: String,
    ntrn_authorizations_addr: String,
    eth_accounts: &[Address],
) -> Result<EthereumProgramLibraries, Box<dyn Error>> {
    info!("Setting up CCTP Transfer on Ethereum");
    let cctp_forwarder_addr = setup_cctp_transfer(
        rt,
        eth_client,
        noble_inbound_ica_addr,
        eth_program_accounts.deposit,
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
    let vault_address = setup_valence_vault(
        rt,
        eth_client,
        eth_strategist_addr,
        eth_accounts,
        eth_admin_addr,
        eth_program_accounts,
        usdc_token_addr,
    )?;

    let libraries = EthereumProgramLibraries {
        cctp_forwarder: cctp_forwarder_addr,
        _lite_processor: lite_processor_address,
        valence_vault: vault_address,
    };

    Ok(libraries)
}

/// sets up a Valence Vault on Ethereum with a proxy.
/// approves deposit & withdraw accounts.
pub fn setup_valence_vault(
    rt: &tokio::runtime::Runtime,
    eth_client: &EthereumClient,
    eth_strategist_acc: Address,
    eth_accounts: &[Address],
    admin: Address,
    eth_program_accounts: EthereumProgramAccounts,
    vault_deposit_token_addr: Address,
) -> Result<Address, Box<dyn Error>> {
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

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
        depositAccount: eth_program_accounts.deposit,
        withdrawAccount: eth_program_accounts.withdraw,
        strategist: eth_strategist_acc,
        fees: fee_config,
        feeDistribution: fee_distribution,
        depositCap: 0, // No cap (for real)
        withdrawLockupPeriod: 1,
        // withdrawLockupPeriod: SECONDS_IN_DAY, // 1 day lockup
        maxWithdrawFeeBps: 10_000, // 1% max withdraw fee
    };

    info!("deploying Valence Vault on Ethereum...");

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

    info!("Approving vault for withdraw account...");
    valence_e2e::utils::ethereum::valence_account::approve_library(
        rt,
        eth_client,
        eth_program_accounts.withdraw,
        proxy_address,
    );

    info!("Approving vault for deposit account...");
    valence_e2e::utils::ethereum::valence_account::approve_library(
        rt,
        eth_client,
        eth_program_accounts.deposit,
        proxy_address,
    );

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
        input_account,
        cctp_address,
    );

    Ok(cctp_address)
}
