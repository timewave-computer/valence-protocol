use std::{
    error::Error,
    path::Path,
    thread::sleep,
    time::{Duration, SystemTime},
};

use localic_utils::{
    types::config::ConfigChain, utils::ethereum::EthClient, ConfigChainBuilder, TestContextBuilder,
    LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};

use log::info;
use neutron::setup_astroport_cl_pool;
use program::{setup_neutron_accounts, setup_neutron_libraries};
use valence_chain_client_utils::{
    cosmos::base_client::BaseClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
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
        solidity_contracts::ValenceVault,
        vault::setup_valence_vault,
        DEFAULT_ANVIL_RPC_ENDPOINT, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH,
        NOBLE_CHAIN_ADMIN_ADDR, NOBLE_CHAIN_DENOM, NOBLE_CHAIN_ID, NOBLE_CHAIN_NAME,
        NOBLE_CHAIN_PREFIX, UUSDC_DENOM, VALENCE_ARTIFACTS_PATH,
    },
};

const EVM_ENCODER_NAMESPACE: &str = "evm_encoder_v1";
const PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";

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

    strategist::swap_ntrn_into_usdc(
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
        &program_hyperlane_contracts
            .eth_hyperlane_contracts
            .mailbox
            .to_string(),
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
