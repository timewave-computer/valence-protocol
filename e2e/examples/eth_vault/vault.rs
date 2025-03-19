use std::{
    error::Error,
    time::{Duration, SystemTime},
};

use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::{bank, cosmwasm::contract_execute};
use localic_utils::{
    types::config::ConfigChain,
    utils::{ethereum::EthClient, test_context::TestContext},
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_ID, NEUTRON_CHAIN_NAME,
};

use log::info;
use neutron::setup_astroport_cl_pool;
use program::{
    setup_astroport_lper_lib, setup_astroport_lwer_lib, setup_cctp_forwarder_lib,
    setup_ica_ibc_transfer_lib, setup_neutron_accounts, setup_neutron_ibc_transfer_lib,
};
use rand::{distributions::Alphanumeric, Rng};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient, evm::request_provider_client::RequestProviderClient,
    neutron::NeutronClient,
};
use valence_e2e::utils::{
    authorization::set_up_authorization_and_processor,
    ethereum::set_up_anvil_container,
    hyperlane::{
        set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts, set_up_hyperlane,
        HyperlaneContracts,
    },
    parse::get_grpc_address_and_port_from_logs,
    ADMIN_MNEMONIC, DEFAULT_ANVIL_RPC_ENDPOINT, ETHEREUM_HYPERLANE_DOMAIN, GAS_FLAGS,
    HYPERLANE_RELAYER_NEUTRON_ADDRESS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH,
    NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME,
    NOBLE_CHAIN_PREFIX, UUSDC_DENOM, VALENCE_ARTIFACTS_PATH,
};
use valence_library_utils::liquidity_utils::AssetData;

const EVM_ENCODER_NAMESPACE: &str = "evm_encoder_v1";
const PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const SECONDS_IN_DAY: u64 = 86_400;

mod ethereum;
mod neutron;
mod noble;
mod program;

/// macro for executing async code in a blocking context
#[macro_export]
macro_rules! async_run {
    ($rt:expr, $($body:tt)*) => {
        $rt.block_on(async { $($body)* })
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;
    async_run!(rt, set_up_anvil_container().await)?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;
    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();

    let eth_accounts = async_run!(rt, eth_client.get_provider_accounts().await.unwrap());
    let eth_admin_acc = eth_accounts[0];
    let eth_user_acc = eth_accounts[2];

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

    let token = create_counterparty_denom(&mut test_ctx)?;
    let uusdc_on_neutron_denom = test_ctx
        .get_ibc_denom()
        .base_denom(UUSDC_DENOM.to_owned())
        .src(NOBLE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let (eth_hyperlane_contracts, ntrn_hyperlane_contracts) =
        hyperlane_plumbing(&mut test_ctx, &eth)?;

    // setup astroport
    let (pool_addr, _lp_token) = setup_astroport_cl_pool(&mut test_ctx, token.to_string())?;

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

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(
            "e2e/examples/eth_vault/neutron_contracts/",
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )?;

    let neutron_program_accounts = setup_neutron_accounts(&mut test_ctx)?;

    let astro_cl_pool_asset_data = AssetData {
        asset1: NEUTRON_CHAIN_DENOM.to_string(),
        asset2: token.to_string(),
    };

    // library to enter into the position from the deposit account
    // and route the issued shares into the into the position account
    let astro_lper_lib = setup_astroport_lper_lib(
        &mut test_ctx,
        neutron_program_accounts.deposit_account.clone(),
        neutron_program_accounts.position_account.clone(),
        astro_cl_pool_asset_data.clone(),
        pool_addr.to_string(),
        neutron_processor_address.to_string(),
    )?;

    // library to withdraw the position held by the position account
    // and route the underlying funds into the withdraw account
    let astro_lwer_lib = setup_astroport_lwer_lib(
        &mut test_ctx,
        neutron_program_accounts.position_account.clone(),
        neutron_program_accounts.withdraw_account.clone(),
        astro_cl_pool_asset_data.clone(),
        pool_addr.to_string(),
        neutron_processor_address.to_string(),
    )?;

    // library to move USDC from a program-owned ICA on noble
    // into the deposit account on neutron
    let ica_ibc_transfer_lib = setup_ica_ibc_transfer_lib(
        &mut test_ctx,
        &neutron_program_accounts
            .noble_inbound_ica
            .library_account
            .to_string()?,
        amount_to_transfer,
    )?;

    // library to move USDC from a program-owned ICA on noble
    // into the withdraw account on ethereum
    let cctp_forwarder_lib_addr = setup_cctp_forwarder_lib(
        &mut test_ctx,
        neutron_program_accounts.withdraw_account.clone(), // replace
        "TODO".to_string(),
        neutron_processor_address,
        amount_to_transfer,
    )?;

    // library to move USDC from the withdraw account on neutron
    // into a program-owned ICA on noble
    let neutron_ibc_transfer_lib = setup_neutron_ibc_transfer_lib(
        &mut test_ctx,
        neutron_program_accounts.withdraw_account,
        valence_library_utils::LibraryAccountType::Addr(
            neutron_program_accounts.noble_outbound_ica.remote_addr,
        ),
        &uusdc_on_neutron_denom,
        amount_to_transfer,
    )?;

    let amount_to_transfer = 1_000_000;

    noble::mint_usdc_to_addr(
        &rt,
        &noble_client,
        &neutron_program_accounts.noble_inbound_ica.remote_addr,
        amount_to_transfer.to_string(),
    )?;

    // Trigger the transfer
    let transfer_msg = &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
        valence_ica_ibc_transfer::msg::FunctionMsgs::Transfer {},
    );

    info!("Executing remote IBC transfer...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &ica_ibc_transfer_lib,
        DEFAULT_KEY,
        &serde_json::to_string(&transfer_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(15));

    // Verify that the funds were successfully sent

    let (grpc_url, grpc_port) = get_grpc_address_and_port_from_logs(NEUTRON_CHAIN_ID)?;
    let neutron_client = async_run!(
        rt,
        NeutronClient::new(
            &grpc_url,
            &grpc_port.to_string(),
            ADMIN_MNEMONIC,
            NEUTRON_CHAIN_ID,
        )
        .await
        .unwrap()
    );

    let balance = async_run!(
        rt,
        neutron_client
            .query_balance(NEUTRON_CHAIN_ADMIN_ADDR, &uusdc_on_neutron_denom)
            .await
            .unwrap()
    );

    assert_eq!(balance, amount_to_transfer);

    info!("Funds successfully sent! ICA IBC Transfer library relayed funds from Noble ICA to Neutron!");

    // setup eth side:
    // 0. encoders
    // 1. lite processor
    // 2. base accounts
    // 3. vault

    // Let's create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    // let deposit_acc_addr =
    //     ethereum::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;
    // let withdraw_acc_addr =
    //     ethereum::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;

    // let usdc_token_address = ethereum::mock_erc20::setup_deposit_erc20(&rt, &eth_client)?;

    // info!("Setting up Lite Processor on Ethereum");
    // let _lite_processor_address = ethereum::lite_processor::setup_lite_processor(
    //     &rt,
    //     &eth_client,
    //     eth_admin_acc,
    //     &eth_hyperlane_contracts.mailbox.to_string(),
    //     authorization_contract_address.as_str(),
    // )?;

    // info!("Setting up Valence Vault...");
    // let vault_address = vault::setup_valence_vault(
    //     &rt,
    //     &eth_client,
    //     &eth_accounts,
    //     eth_admin_acc,
    //     deposit_acc_addr,
    //     withdraw_acc_addr,
    //     usdc_token_address,
    // )?;

    // let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    // let usdc_token = MockERC20::new(usdc_token_address, &eth_rp);
    // let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    // info!("funding eth user with 1_000_000USDC...");
    // ethereum::mock_erc20::mint(
    //     &rt,
    //     &eth_client,
    //     usdc_token_address,
    //     eth_user_acc,
    //     U256::from(1_000_000),
    // );

    // info!("approving vault to spend usdc on behalf of user...");
    // ethereum::mock_erc20::approve(
    //     &rt,
    //     &eth_client,
    //     usdc_token_address,
    //     eth_user_acc,
    //     *valence_vault.address(),
    //     U256::MAX,
    // );

    // info!("Approving vault for deposit account...");
    // ethereum::valence_account::approve_library(
    //     &rt,
    //     &eth_client,
    //     deposit_acc_addr,
    //     *valence_vault.address(),
    // );
    // info!("Approving vault for withdraw account...");
    // ethereum::valence_account::approve_library(
    //     &rt,
    //     &eth_client,
    //     withdraw_acc_addr,
    //     *valence_vault.address(),
    // );

    // vault::query_vault_config(*valence_vault.address(), &rt, &eth_client);
    // let vault_total_assets =
    //     vault::query_vault_total_assets(*valence_vault.address(), &rt, &eth_client);
    // let vault_total_supply =
    //     vault::query_vault_total_supply(*valence_vault.address(), &rt, &eth_client);
    // let user_vault_bal =
    //     vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user_acc);

    // info!("vault total assets: {:?}", vault_total_assets._0);
    // info!("vault total supply: {:?}", vault_total_supply._0);
    // info!("user vault balance: {:?}", user_vault_bal._0);

    // info!("Approving token for vault...");
    // ethereum::mock_erc20::approve(
    //     &rt,
    //     &eth_client,
    //     usdc_token_address,
    //     eth_admin_acc,
    //     *valence_vault.address(),
    //     U256::MAX,
    // );

    // let deposit_amount = U256::from(500_000);

    // let vault_state = vault::query_vault_packed_values(*valence_vault.address(), &rt, &eth_client);
    // info!("vault packed values: {:?}", vault_state);

    // info!("User depositing {deposit_amount}USDC tokens to vault...");
    // vault::deposit_to_vault(
    //     &rt,
    //     &eth_client,
    //     *valence_vault.address(),
    //     eth_user_acc,
    //     deposit_amount,
    // )?;

    // log_eth_balances(
    //     &eth_client,
    //     &rt,
    //     valence_vault.address(),
    //     &usdc_token_address,
    //     &deposit_acc_addr,
    //     &withdraw_acc_addr,
    //     &eth_user_acc,
    // )
    // .unwrap();

    // let current_rate = vault::query_redemption_rate(*valence_vault.address(), &rt, &eth_client)._0;
    // let netting_amount = U256::from(0);
    // let withdraw_fee_bps = 1;

    // info!("performing vault update...");
    // vault::vault_update(
    //     *valence_vault.address(),
    //     current_rate,
    //     withdraw_fee_bps,
    //     netting_amount,
    //     &rt,
    //     &eth_client,
    // )?;

    // info!("pausing the vault...");
    // vault::pause(*valence_vault.address(), &rt, &eth_client)?;

    // info!("attempting to deposit to paused vault...");
    // vault::deposit_to_vault(
    //     &rt,
    //     &eth_client,
    //     *valence_vault.address(),
    //     eth_user_acc,
    //     deposit_amount,
    // )?;

    // info!("resuming the vault...");
    // vault::unpause(*valence_vault.address(), &rt, &eth_client)?;

    // info!("attempting to deposit to active vault...");
    // vault::deposit_to_vault(
    //     &rt,
    //     &eth_client,
    //     *valence_vault.address(),
    //     eth_user_acc,
    //     deposit_amount,
    // )?;

    // log_eth_balances(
    //     &eth_client,
    //     &rt,
    //     valence_vault.address(),
    //     &usdc_token_address,
    //     &deposit_acc_addr,
    //     &withdraw_acc_addr,
    //     &eth_user_acc,
    // )?;

    // info!("minting some USDC for admin...");
    // ethereum::mock_erc20::mint(
    //     &rt,
    //     &eth_client,
    //     usdc_token_address,
    //     eth_admin_acc,
    //     deposit_amount * U256::from(5),
    // );

    // info!("transferring USDC from admin to withdraw account...");
    // ethereum::mock_erc20::transfer(
    //     &rt,
    //     &eth_client,
    //     usdc_token_address,
    //     eth_admin_acc,
    //     withdraw_acc_addr,
    //     deposit_amount * U256::from(5),
    // );

    // async_run!(rt, {
    //     let withdraw_account = BaseAccount::new(withdraw_acc_addr, &eth_rp);

    //     let approve_calldata = usdc_token
    //         .approve(*valence_vault.address(), U256::MAX)
    //         .calldata()
    //         .clone();

    //     eth_client
    //         .execute_tx(
    //             withdraw_account
    //                 .execute(usdc_token_address, U256::from(0), approve_calldata)
    //                 .into_transaction_request(),
    //         )
    //         .await
    //         .unwrap();

    //     let allowance = eth_client
    //         .query(usdc_token.allowance(withdraw_acc_addr, *valence_vault.address()))
    //         .await
    //         .unwrap();

    //     info!("Withdraw account has approved vault for: {}", allowance._0);

    //     info!("asserting that vault is approved by the withdraw account...");

    //     let withdraw_account = BaseAccount::new(withdraw_acc_addr, &eth_rp);

    //     let is_approved = eth_client
    //         .query(withdraw_account.approvedLibraries(*valence_vault.address()))
    //         .await
    //         .unwrap();

    //     info!(
    //         "vault approved as library for withdraw account: {}",
    //         is_approved._0
    //     );
    //     let bal = eth_client
    //         .query(usdc_token.balanceOf(withdraw_acc_addr))
    //         .await
    //         .unwrap()
    //         ._0;
    //     info!("ETH WITHDRAW ACC USDC BAL\t: {bal}");
    // });

    // log_eth_balances(
    //     &eth_client,
    //     &rt,
    //     valence_vault.address(),
    //     &usdc_token_address,
    //     &deposit_acc_addr,
    //     &withdraw_acc_addr,
    //     &eth_user_acc,
    // )
    // .unwrap();

    // info!("User initiates shares redeemal...");

    // let user_shares =
    //     vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user_acc)._0;

    // vault::redeem(
    //     *valence_vault.address(),
    //     &rt,
    //     &eth_client,
    //     eth_user_acc,
    //     user_shares,
    //     10_000,
    //     true,
    // )?;

    // let has_active_withdraw =
    //     vault::addr_has_active_withdraw(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    // info!("user active withdraws: {:?}", has_active_withdraw._0);

    // let user_withdraw_request =
    //     vault::addr_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    // info!("user active withdraw request: {:?}", user_withdraw_request);

    // log_eth_balances(
    //     &eth_client,
    //     &rt,
    //     valence_vault.address(),
    //     &usdc_token_address,
    //     &deposit_acc_addr,
    //     &withdraw_acc_addr,
    //     &eth_user_acc,
    // )
    // .unwrap();

    // sleep(Duration::from_secs(3));

    // info!("user attempts to finalize the withdrawal...");
    // vault::complete_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc)?;

    // let has_active_withdraw =
    //     vault::addr_has_active_withdraw(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    // info!("user active withdraws: {:?}", has_active_withdraw._0);

    // let user_withdraw_request =
    //     vault::addr_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    // info!("user active withdraw request: {:?}", user_withdraw_request);

    // log_eth_balances(
    //     &eth_client,
    //     &rt,
    //     valence_vault.address(),
    //     &usdc_token_address,
    //     &deposit_acc_addr,
    //     &withdraw_acc_addr,
    //     &eth_user_acc,
    // )
    // .unwrap();

    Ok(())
}

fn create_counterparty_denom(test_ctx: &mut TestContext) -> Result<String, Box<dyn Error>> {
    info!("creating subdenom to pair with NTRN");
    // Let's create a token to pair it with NTRN
    let token_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token_subdenom)
        .get();

    // Mint some of the token
    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(1_000_000_000)
        .with_denom(&token)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok(token)
}

fn hyperlane_plumbing(
    test_ctx: &mut TestContext,
    eth: &EthClient,
) -> Result<(HyperlaneContracts, HyperlaneContracts), Box<dyn Error>> {
    info!("uploading cosmwasm hyperlane contracts...");
    // Upload all Hyperlane contracts to Neutron
    let neutron_hyperlane_contracts = set_up_cw_hyperlane_contracts(test_ctx)?;

    info!("uploading evm hyperlane conrtacts...");
    // Deploy all Hyperlane contracts on Ethereum
    let eth_hyperlane_contracts = set_up_eth_hyperlane_contracts(eth, ETHEREUM_HYPERLANE_DOMAIN)?;

    info!("setting up hyperlane connection Neutron <> Ethereum");
    set_up_hyperlane(
        "hyperlane-net",
        vec!["localneutron-1-val-0-neutronic", "anvil"],
        "neutron",
        "ethereum",
        &neutron_hyperlane_contracts,
        &eth_hyperlane_contracts,
    )?;

    // Since we are going to relay callbacks to Neutron, let's fund the Hyperlane relayer account with some tokens
    info!("Fund relayer account...");
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        HYPERLANE_RELAYER_NEUTRON_ADDRESS,
        &[BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: 5_000_000u128.into(),
        }],
        &BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok((eth_hyperlane_contracts, neutron_hyperlane_contracts))
}
