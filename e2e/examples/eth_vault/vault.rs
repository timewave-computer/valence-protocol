use std::{
    error::Error,
    path::Path,
    thread::sleep,
    time::{Duration, SystemTime},
};

use alloy::primitives::U256;
use evm::log_eth_balances;
use localic_utils::{
    types::config::ConfigChain, utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};

use log::info;
use neutron::setup_astroport_cl_pool;
use program::{setup_neutron_accounts, setup_neutron_libraries};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient, evm::request_provider_client::RequestProviderClient,
};

use valence_e2e::{
    async_run,
    utils::{
        authorization::set_up_authorization_and_processor,
        ethereum as ethereum_utils,
        manager::{
            ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME, BASE_ACCOUNT_NAME,
            ICA_CCTP_TRANSFER_NAME, ICA_IBC_TRANSFER_NAME, INTERCHAIN_ACCOUNT_NAME,
            NEUTRON_IBC_TRANSFER_NAME,
        },
        mock_cctp_relayer,
        solidity_contracts::ValenceVault,
        vault::{self, setup_cctp_transfer, setup_valence_vault},
        DEFAULT_ANVIL_RPC_ENDPOINT, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH,
        NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME,
        NOBLE_CHAIN_PREFIX, UUSDC_DENOM, VALENCE_ARTIFACTS_PATH,
    },
};

const _PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const _WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const VAULT_NEUTRON_CACHE_PATH: &str = "e2e/examples/eth_vault/neutron_contracts/";

mod evm;
mod neutron;
mod noble;
mod program;
mod strategist;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;

    info!("Initializing ethereum side flow...");
    async_run!(rt, ethereum_utils::set_up_anvil_container().await)?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;

    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();
    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let eth_accounts = async_run!(rt, eth_client.get_provider_accounts().await.unwrap());
    let eth_admin_acc = eth_accounts[0];
    let _eth_user_acc = eth_accounts[2];
    let eth_cctp_relay_acc = eth_accounts[5];

    // create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    let deposit_acc_addr =
        ethereum_utils::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;
    let withdraw_acc_addr =
        ethereum_utils::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;
    // set up the cctp messenger
    let mock_cctp_messenger_address =
        valence_e2e::utils::vault::setup_mock_token_messenger(&rt, &eth_client)?;

    info!("Setting up Neutron side flow...");

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_chain(ConfigChain {
            denom: NOBLE_CHAIN_DENOM.to_string(),
            debugging: true,
            chain_id: NOBLE_CHAIN_ID.to_string(),
            chain_name: NOBLE_CHAIN_NAME.to_string(),
            chain_prefix: NOBLE_CHAIN_PREFIX.to_string(),
            admin_addr: NOBLE_CHAIN_ADMIN_ADDR.to_string(),
        })
        .with_log_file_path(LOGS_FILE_PATH)
        .with_transfer_channels(NEUTRON_CHAIN_NAME, NOBLE_CHAIN_NAME)
        .build()?;

    let noble_client = noble::get_client(&rt)?;
    noble::setup_environment(&rt, &noble_client)?;
    noble::mint_usdc_to_addr(&rt, &noble_client, NOBLE_CHAIN_ADMIN_ADDR, 999900000)?;

    async_run!(&rt, {
        let rx = noble_client
            .ibc_transfer(
                NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
                UUSDC_DENOM.to_string(),
                999000000.to_string(),
                test_ctx
                    .get_transfer_channels()
                    .src(NOBLE_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
                60,
                None,
            )
            .await
            .unwrap();
        noble_client.poll_for_tx(&rx.hash).await.unwrap();
    });

    sleep(Duration::from_secs(3));

    let uusdc_on_neutron_denom = test_ctx
        .get_ibc_denom()
        .base_denom(UUSDC_DENOM.to_owned())
        .src(NOBLE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let program_hyperlane_contracts = utils::hyperlane_plumbing(&mut test_ctx, &eth)?;

    // setup astroport
    let (pool_addr, lp_token) =
        setup_astroport_cl_pool(&mut test_ctx, uusdc_on_neutron_denom.to_string())?;

    let salt = hex::encode(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            .to_string(),
    );
    let amount_to_transfer = 1_000_000;

    // set up the authorization and processor contracts on neutron
    let (authorization_contract_address, neutron_processor_address) =
        set_up_authorization_and_processor(&mut test_ctx, salt.clone())?;

    // copy over relevant contracts from artifacts/ to local path
    let local_contracts_path = Path::new(VAULT_NEUTRON_CACHE_PATH);
    if !local_contracts_path.exists() {
        std::fs::create_dir(local_contracts_path)?;
    }

    for contract in [
        INTERCHAIN_ACCOUNT_NAME,
        ASTROPORT_LPER_NAME,
        ASTROPORT_WITHDRAWER_NAME,
        NEUTRON_IBC_TRANSFER_NAME,
        ICA_CCTP_TRANSFER_NAME,
        ICA_IBC_TRANSFER_NAME,
        BASE_ACCOUNT_NAME,
    ] {
        let contract_name = format!("{}.wasm", contract);
        let contract_path = Path::new(&contract_name);
        let src = Path::new("artifacts/").join(contract_path);
        let dest = local_contracts_path.join(contract_path);
        std::fs::copy(src, dest)?;
    }

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(
            "e2e/examples/eth_vault/neutron_contracts/",
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )?;

    let neutron_program_accounts = setup_neutron_accounts(&mut test_ctx)?;

    let neutron_program_libraries = setup_neutron_libraries(
        &mut test_ctx,
        &neutron_program_accounts,
        &pool_addr,
        &neutron_processor_address,
        amount_to_transfer,
        &uusdc_on_neutron_denom,
        eth_admin_acc.to_string(),
        withdraw_acc_addr.to_string(),
    )?;

    noble::mint_usdc_to_addr(
        &rt,
        &noble_client,
        &neutron_program_accounts.noble_inbound_ica.remote_addr,
        amount_to_transfer,
    )?;

    let neutron_client = neutron::get_neutron_client(&rt)?;

    strategist::pull_funds_from_noble_inbound_ica(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &neutron_program_libraries,
        &uusdc_on_neutron_denom,
        amount_to_transfer,
    )?;

    strategist::enter_position(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &neutron_program_libraries,
        &uusdc_on_neutron_denom,
        &lp_token,
    )?;

    strategist::exit_position(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &neutron_program_libraries,
        &uusdc_on_neutron_denom,
        &lp_token,
    )?;

    strategist::swap_ntrn_into_usdc(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &uusdc_on_neutron_denom,
        &pool_addr,
    )?;

    let usdc_token_address =
        ethereum_utils::mock_erc20::setup_deposit_erc20(&rt, &eth_client, "MockUSDC", "USDC")?;

    info!("Starting CCTP mock relayer between Noble and Ethereum...");
    let mock_cctp_relayer = mock_cctp_relayer::MockCctpRelayer::new(&rt);
    let rly_rt = tokio::runtime::Runtime::new().unwrap();

    let _join_handle = rly_rt
        .spawn(mock_cctp_relayer.start_relay(usdc_token_address, mock_cctp_messenger_address));
    info!("main sleep for 3...");
    sleep(Duration::from_secs(3));

    strategist::route_usdc_to_noble(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &neutron_program_libraries,
        &uusdc_on_neutron_denom,
    )?;

    let noble_outbound_ica_usdc_bal = async_run!(
        &rt,
        noble_client
            .query_balance(
                &neutron_program_accounts.noble_outbound_ica.remote_addr,
                UUSDC_DENOM
            )
            .await
            .unwrap()
    );
    info!("noble_outbound_ica_usdc_bal: {noble_outbound_ica_usdc_bal}");

    strategist::cctp_route_usdc_from_noble(
        &rt,
        &neutron_client,
        &noble_client,
        &neutron_program_accounts,
        &neutron_program_libraries,
    )?;

    info!("Setting up Lite Processor on Ethereum");
    let _lite_processor_address = ethereum_utils::lite_processor::setup_lite_processor(
        &rt,
        &eth_client,
        eth_admin_acc,
        &program_hyperlane_contracts
            .eth_hyperlane_contracts
            .mailbox
            .to_string(),
        authorization_contract_address.as_str(),
    )?;

    info!("Setting up Valence Vault...");
    let vault_address = setup_valence_vault(
        &rt,
        &eth_client,
        &eth_accounts,
        eth_admin_acc,
        deposit_acc_addr,
        withdraw_acc_addr,
        usdc_token_address,
    )?;

    let cctp_forwarder = setup_cctp_transfer(
        &rt,
        &eth_client,
        neutron_program_accounts
            .noble_inbound_ica
            .remote_addr
            .to_string(),
        deposit_acc_addr,
        eth_admin_acc,
        eth_admin_acc,
        usdc_token_address,
        mock_cctp_messenger_address,
    )?;

    // approve the CCTP forwarder on deposit account
    ethereum_utils::valence_account::approve_library(
        &rt,
        &eth_client,
        deposit_acc_addr,
        cctp_forwarder,
    );

    let eth_user_acc = eth_accounts[2];
    let eth_user2_acc = eth_accounts[3];

    let user_1_deposit_amount = U256::from(500_000);
    let user_2_deposit_amount = U256::from(1_000_000);

    info!("funding eth user with {user_1_deposit_amount}USDC...");
    ethereum_utils::mock_erc20::mint(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user_acc,
        user_1_deposit_amount,
    );

    let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    info!("approving vault to spend usdc on behalf of user...");
    ethereum_utils::mock_erc20::approve(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user_acc,
        *valence_vault.address(),
        U256::MAX,
    );

    info!("Approving vault for withdraw account...");
    ethereum_utils::valence_account::approve_library(
        &rt,
        &eth_client,
        withdraw_acc_addr,
        *valence_vault.address(),
    );

    info!("Approving vault for deposit account...");
    ethereum_utils::valence_account::approve_library(
        &rt,
        &eth_client,
        deposit_acc_addr,
        *valence_vault.address(),
    );

    info!("User depositing {user_1_deposit_amount}USDC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_user_acc,
        user_1_deposit_amount,
    )?;

    let current_rate = vault::query_redemption_rate(*valence_vault.address(), &rt, &eth_client)._0;
    let netting_amount = U256::from(0);
    let withdraw_fee_bps = 1;

    info!("performing vault update...");
    vault::vault_update(
        *valence_vault.address(),
        current_rate,
        withdraw_fee_bps,
        netting_amount,
        &rt,
        &eth_client,
    )?;

    info!("funding eth user2 with {user_2_deposit_amount}USDC...");
    ethereum_utils::mock_erc20::mint(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user2_acc,
        user_2_deposit_amount,
    );

    info!("approving vault to spend usdc on behalf of user2...");
    ethereum_utils::mock_erc20::approve(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user2_acc,
        *valence_vault.address(),
        U256::MAX,
    );

    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        alloy::providers::ext::AnvilApi::anvil_mine(
            &eth_rp,
            Some(U256::from(5)),
            Some(U256::from(3)),
        )
        .await
        .unwrap();
    });

    let user1_pre_redeem_shares_bal =
        vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user_acc)._0;
    assert_ne!(user1_pre_redeem_shares_bal, U256::ZERO);

    info!("USER1 initiating the redeem of {user1_pre_redeem_shares_bal} shares from vault...");
    vault::redeem(
        vault_address,
        &rt,
        &eth_client,
        eth_user_acc,
        user1_pre_redeem_shares_bal,
        10_000,
        true,
    )?;
    let user1_post_redeem_shares_bal =
        vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user_acc)._0;
    assert_eq!(user1_post_redeem_shares_bal, U256::ZERO);

    let has_active_withdraw =
        vault::addr_has_active_withdraw(*valence_vault.address(), &rt, &eth_client, eth_user_acc)
            ._0;
    assert!(has_active_withdraw);

    info!("User2 depositing {user_2_deposit_amount}USDC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_user2_acc,
        U256::from(1_000_000),
    )?;
    let user2_shares_bal =
        vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user2_acc)._0;
    let user2_post_deposit_usdc_bal = ethereum_utils::mock_erc20::query_balance(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user2_acc,
    );
    assert_ne!(user2_shares_bal, U256::ZERO);
    assert_eq!(user2_post_deposit_usdc_bal, U256::ZERO);

    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        alloy::providers::ext::AnvilApi::anvil_mine(
            &eth_rp,
            Some(U256::from(5)),
            Some(U256::from(3)),
        )
        .await
        .unwrap();
    });

    info!("performing vault update with N=100_000...");
    vault::vault_update(
        *valence_vault.address(),
        current_rate,
        withdraw_fee_bps,
        // netting the full amount
        user_1_deposit_amount,
        &rt,
        &eth_client,
    )?;

    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        alloy::providers::ext::AnvilApi::anvil_mine(
            &eth_rp,
            Some(U256::from(5)),
            Some(U256::from(3)),
        )
        .await
        .unwrap();
    });

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
        &eth_user2_acc,
    )
    .unwrap();

    info!("user1 completing withdraw request...");
    vault::complete_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc)?;
    let withdraw_acc_usdc_bal = ethereum_utils::mock_erc20::query_balance(
        &rt,
        &eth_client,
        usdc_token_address,
        withdraw_acc_addr,
    );
    let user1_usdc_bal = ethereum_utils::mock_erc20::query_balance(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user_acc,
    );
    assert_eq!(user1_usdc_bal, user_1_deposit_amount - U256::from(50));

    info!("strategist cctp routing eth->ntrn...");
    strategist::cctp_route_usdc_from_eth(&rt, &eth_client, cctp_forwarder, eth_admin_acc)?;

    info!("[MAIN] sleeping for 5 to give cctp time to relay");
    sleep(Duration::from_secs(5));

    let deposit_acc_usdc_bal = ethereum_utils::mock_erc20::query_balance(
        &rt,
        &eth_client,
        usdc_token_address,
        deposit_acc_addr,
    );
    // assert_eq!(deposit_acc_usdc_bal, U256::ZERO);

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
        &eth_user2_acc,
    )
    .unwrap();

    sleep(Duration::from_secs(5));

    Ok(())
}
