use std::{
    error::Error,
    path::Path,
    thread::sleep,
    time::{Duration, SystemTime},
};

use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::bank;
use localic_utils::{
    types::config::ConfigChain,
    utils::{ethereum::EthClient, test_context::TestContext},
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};

use log::info;
use neutron::setup_astroport_cl_pool;
use program::{setup_neutron_accounts, setup_neutron_libraries};
use rand::{distributions::Alphanumeric, Rng};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    authorization::set_up_authorization_and_processor,
    ethereum as ethereum_utils,
    hyperlane::{
        set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts, set_up_hyperlane,
        HyperlaneContracts,
    },
    manager::{
        ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME, BASE_ACCOUNT_NAME, ICA_CCTP_TRANSFER_NAME,
        ICA_IBC_TRANSFER_NAME, INTERCHAIN_ACCOUNT_NAME, NEUTRON_IBC_TRANSFER_NAME,
    },
    solidity_contracts::ValenceVault,
    vault::setup_valence_vault,
    DEFAULT_ANVIL_RPC_ENDPOINT, ETHEREUM_HYPERLANE_DOMAIN, HYPERLANE_RELAYER_NEUTRON_ADDRESS,
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM,
    NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME, NOBLE_CHAIN_PREFIX, UUSDC_DENOM, VALENCE_ARTIFACTS_PATH,
};

const EVM_ENCODER_NAMESPACE: &str = "evm_encoder_v1";
const PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const _SECONDS_IN_DAY: u64 = 86_400;

mod evm;
mod neutron;
mod noble;
mod program;
mod strategist;

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
    noble::fund_neutron_addr(
        &rt,
        &mut test_ctx,
        &noble_client,
        NEUTRON_CHAIN_ADMIN_ADDR,
        999000000,
    )?;

    sleep(Duration::from_secs(3));

    let uusdc_on_neutron_denom = test_ctx
        .get_ibc_denom()
        .base_denom(UUSDC_DENOM.to_owned())
        .src(NOBLE_CHAIN_NAME)
        .dest(NEUTRON_CHAIN_NAME)
        .get();

    let (eth_hyperlane_contracts, _ntrn_hyperlane_contracts) =
        hyperlane_plumbing(&mut test_ctx, &eth)?;

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
    // std::fs::copy(, to)
    let local_contracts_path = Path::new("e2e/examples/eth_vault/neutron_contracts/");
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
        info!("src path: {:?}", src);
        info!("dest path: {:?}", dest);
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

    strategist::swap_counterparty_denom_into_usdc(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &uusdc_on_neutron_denom,
        &pool_addr,
    )?;

    strategist::route_usdc_to_noble(
        &rt,
        &neutron_client,
        &neutron_program_accounts,
        &neutron_program_libraries,
        &uusdc_on_neutron_denom,
    )?;

    sleep(Duration::from_secs(5));

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

    // create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    let deposit_acc_addr =
        ethereum_utils::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;
    let withdraw_acc_addr =
        ethereum_utils::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;

    let usdc_token_address =
        ethereum_utils::mock_erc20::setup_deposit_erc20(&rt, &eth_client, "MockUSDC", "USDC")?;

    info!("Setting up Lite Processor on Ethereum");
    let _lite_processor_address = ethereum_utils::lite_processor::setup_lite_processor(
        &rt,
        &eth_client,
        eth_admin_acc,
        &eth_hyperlane_contracts.mailbox.to_string(),
        authorization_contract_address.as_str(),
    )?;

    let _mock_cctp_messenger_address =
        valence_e2e::utils::vault::setup_mock_token_messenger(&rt, &eth_client)?;

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

    let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    let vault_config = async_run!(&rt, eth_client.query(valence_vault.config()).await.unwrap());

    info!("instantiated valence vault config: {:?}", vault_config);

    Ok(())
}

#[allow(unused)]
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
