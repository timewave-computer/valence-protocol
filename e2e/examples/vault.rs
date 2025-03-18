use std::{collections::HashMap, env, error::Error, str::FromStr, thread::sleep, time::Duration};

use alloy::primitives::{Address, U256};
use cosmwasm_std::{coin, to_json_binary, Binary, Decimal, Empty};
use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate, contract_query},
};
use localic_utils::{
    utils::{ethereum::EthClient, test_context::TestContext},
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};

use log::{info, warn};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::Value;
use valence_astroport_lper::msg::LiquidityProviderConfig;
use valence_astroport_utils::astroport_native_lp_token::{
    Asset, AssetInfo, ConcentratedLiquidityExecuteMsg, ConcentratedPoolParams,
    FactoryInstantiateMsg, FactoryQueryMsg, NativeCoinRegistryExecuteMsg,
    NativeCoinRegistryInstantiateMsg, PairConfig, PairType,
};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
    msg::{
        EncoderInfo, EvmBridgeInfo, ExternalDomainInfo, HyperlaneConnectorInfo, PermissionedMsg,
        ProcessorMessage,
    },
};
use valence_chain_client_utils::{
    ethereum::EthereumClient,
    evm::{base_client::EvmBaseClient, request_provider_client::RequestProviderClient},
};
use valence_e2e::utils::{
    ethereum::{self, set_up_anvil_container},
    hyperlane::{
        set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts, set_up_hyperlane,
        HyperlaneContracts,
    },
    manager::{setup_manager, use_manager_init, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME},
    processor::tick_processor,
    solidity_contracts::{
        BaseAccount, MockERC20,
        ValenceVault::{self},
    },
    vault, ASTROPORT_PATH, DEFAULT_ANVIL_RPC_ENDPOINT, ETHEREUM_CHAIN_NAME,
    ETHEREUM_HYPERLANE_DOMAIN, GAS_FLAGS, HYPERLANE_RELAYER_NEUTRON_ADDRESS,
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use valence_library_utils::liquidity_utils::AssetData;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::LibraryInfo,
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

const EVM_ENCODER_NAMESPACE: &str = "evm_encoder_v1";
const PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "provide_liquidity";
const WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL: &str = "withdraw_liquidity";
const ASTROPORT_CONCENTRATED_PAIR_TYPE: &str = "concentrated";
const SECONDS_IN_DAY: u64 = 86_400;

/// macro for executing async code in a blocking context
macro_rules! async_run {
    ($rt:expr, $($body:tt)*) => {
        $rt.block_on(async { $($body)* })
    }
}

pub fn my_evm_vault_program(
    ntrn_domain: valence_program_manager::domain::Domain,
    asset_1: &str,
    asset_2: &str,
    pool_addr: &str,
    owner: &str,
) -> Result<ProgramConfig, Box<dyn Error>> {
    let mut builder = ProgramConfigBuilder::new("vault test", owner);

    let deposit_account_info =
        AccountInfo::new("deposit".to_string(), &ntrn_domain, AccountType::default());

    let position_account_info =
        AccountInfo::new("position".to_string(), &ntrn_domain, AccountType::default());

    let withdraw_account_info =
        AccountInfo::new("withdraw".to_string(), &ntrn_domain, AccountType::default());

    let deposit_acc = builder.add_account(deposit_account_info);
    let position_acc = builder.add_account(position_account_info);
    let withdraw_acc = builder.add_account(withdraw_account_info);

    let astro_cl_pair_type = valence_astroport_utils::astroport_native_lp_token::PairType::Custom(
        ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string(),
    );

    let astro_cl_pool_asset_data = AssetData {
        asset1: asset_1.to_string(),
        asset2: asset_2.to_string(),
    };

    let astro_lp_config = LiquidityProviderConfig {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(astro_cl_pair_type.clone()),
        asset_data: astro_cl_pool_asset_data.clone(),
        max_spread: None,
    };

    let astro_lw_config = valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(astro_cl_pair_type),
        asset_data: astro_cl_pool_asset_data.clone(),
    };

    let astro_lper_library_cfg = valence_astroport_lper::msg::LibraryConfig {
        input_addr: deposit_acc.clone(),
        output_addr: position_acc.clone(),
        lp_config: astro_lp_config,
        pool_addr: pool_addr.to_string(),
    };
    let astro_lwer_library_cfg = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: position_acc.clone(),
        output_addr: withdraw_acc.clone(),
        withdrawer_config: astro_lw_config,
        pool_addr: pool_addr.to_string(),
    };

    let astro_lper_library = builder.add_library(LibraryInfo::new(
        "astro_lp".to_string(),
        &ntrn_domain,
        valence_program_manager::library::LibraryConfig::ValenceAstroportLper(
            astro_lper_library_cfg,
        ),
    ));

    let astro_lwer_library = builder.add_library(LibraryInfo::new(
        "astro_lw".to_string(),
        &ntrn_domain,
        valence_program_manager::library::LibraryConfig::ValenceAstroportWithdrawer(
            astro_lwer_library_cfg,
        ),
    ));

    // establish the deposit_acc -> lper_lib -> position_acc link
    builder.add_link(&astro_lper_library, vec![&deposit_acc], vec![&position_acc]);
    // establish the position_acc -> lwer_lib -> withdraw_acc link
    builder.add_link(
        &astro_lwer_library,
        vec![&position_acc],
        vec![&withdraw_acc],
    );

    let astro_lper_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::Main)
        .with_contract_address(astro_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let astro_lwer_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::Main)
        .with_contract_address(astro_lwer_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let astro_lper_subroutine = AtomicSubroutineBuilder::new()
        .with_function(astro_lper_function)
        .build();

    let astro_lwer_subroutine = AtomicSubroutineBuilder::new()
        .with_function(astro_lwer_function)
        .build();

    let astro_lper_authorization = AuthorizationBuilder::new()
        .with_label(PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL)
        .with_subroutine(astro_lper_subroutine)
        .build();
    let astro_lwer_authorization = AuthorizationBuilder::new()
        .with_label(WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL)
        .with_subroutine(astro_lwer_subroutine)
        .build();

    builder.add_authorization(astro_lper_authorization);
    builder.add_authorization(astro_lwer_authorization);

    let program_config = builder.build();

    Ok(program_config)
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Start anvil container
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(set_up_anvil_container())?;

    let eth = EthClient::new(DEFAULT_ANVIL_RPC_ENDPOINT)?;
    let eth_client = valence_chain_client_utils::ethereum::EthereumClient::new(
        DEFAULT_ANVIL_RPC_ENDPOINT,
        "test test test test test test test test test test test junk",
    )
    .unwrap();

    let eth_accounts = async_run!(rt, { eth_client.get_provider_accounts().await.unwrap() });
    let eth_admin_acc = eth_accounts[0];
    let eth_user_acc = eth_accounts[2];

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    let token = create_counterparty_denom(&mut test_ctx)?;

    let (eth_hyperlane_contracts, ntrn_hyperlane_contracts) =
        hyperlane_plumbing(&mut test_ctx, &eth)?;

    // setup astroport
    let (
        astroport_factory_code_id,
        astroport_pair_concentrated_code_id,
        astroport_token_code_id,
        astroport_coin_registry_code_id,
    ) = deploy_astroport_contracts(&mut test_ctx)?;

    let (pool_addr, _lp_token) = setup_astroport_cl_pool(
        &mut test_ctx,
        astroport_pair_concentrated_code_id,
        astroport_token_code_id,
        astroport_factory_code_id,
        astroport_coin_registry_code_id,
        token.to_string(),
    )?;

    // setup neutron side:
    // 1. authorizations
    // 2. processor
    // 3. astroport LP & LW
    // 4. base account
    setup_manager(
        &mut test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME],
    )?;

    let ntrn_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let mut program_config = my_evm_vault_program(
        ntrn_domain.clone(),
        NEUTRON_CHAIN_DENOM,
        &token,
        &pool_addr,
        NEUTRON_CHAIN_ADMIN_ADDR,
    )?;

    info!("initializing manager...");
    use_manager_init(&mut program_config)?;

    info!("fetching manager build artifacts...");
    let deposit_acc_addr = program_config.get_account(0u64)?.addr.clone().unwrap();
    let position_acc_addr = program_config.get_account(1u64)?.addr.clone().unwrap();
    let withdraw_acc_addr = program_config.get_account(2u64)?.addr.clone().unwrap();
    let authorization_contract_address = program_config
        .authorization_data
        .authorization_addr
        .to_string();
    let ntrn_processor_contract_address = program_config
        .get_processor_addr(&ntrn_domain.to_string())
        .unwrap();

    info!("NTRN DEPOSIT ACC\t: {deposit_acc_addr}");
    info!("NTRN POSITION ACC\t: {position_acc_addr}");
    info!("NTRN WITHDRAW ACC\t: {withdraw_acc_addr}");
    info!("NTRN AUTHORIZATIONS\t: {authorization_contract_address}");
    info!("NTRN PROCESSOR\t: {ntrn_processor_contract_address}");

    // info!("Setting up encoders ...");
    // let evm_encoder = setup_valence_evm_encoder_v1(&mut test_ctx)?;
    // let encoder_broker = setup_valence_encoder_broker(&mut test_ctx, evm_encoder.to_string())?;

    // setup eth side:
    // 0. encoders
    // 1. lite processor
    // 2. base accounts
    // 3. vault

    // Let's create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    let deposit_acc_addr =
        ethereum::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;
    let withdraw_acc_addr =
        ethereum::valence_account::setup_valence_account(&rt, &eth_client, eth_admin_acc)?;

    let usdc_token_address = ethereum::mock_erc20::setup_deposit_erc20(&rt, &eth_client)?;

    info!("Setting up Lite Processor on Ethereum");
    let _lite_processor_address = ethereum::lite_processor::setup_lite_processor(
        &rt,
        &eth_client,
        eth_admin_acc,
        &eth_hyperlane_contracts.mailbox.to_string(),
        authorization_contract_address.as_str(),
    )?;

    info!("Setting up Valence Vault...");
    let vault_address = vault::setup_valence_vault(
        &rt,
        &eth_client,
        &eth_accounts,
        eth_admin_acc,
        deposit_acc_addr,
        withdraw_acc_addr,
        usdc_token_address,
    )?;

    let eth_rp = async_run!(rt, eth_client.get_request_provider().await.unwrap());

    let usdc_token = MockERC20::new(usdc_token_address, &eth_rp);
    let valence_vault = ValenceVault::new(vault_address, &eth_rp);

    info!("funding eth user with 1_000_000USDC...");
    ethereum::mock_erc20::mint(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user_acc,
        U256::from(1_000_000),
    );

    info!("approving vault to spend usdc on behalf of user...");
    ethereum::mock_erc20::approve(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_user_acc,
        *valence_vault.address(),
        U256::MAX,
    );

    info!("Approving vault for deposit account...");
    ethereum::valence_account::approve_library(
        &rt,
        &eth_client,
        deposit_acc_addr,
        *valence_vault.address(),
    );
    info!("Approving vault for withdraw account...");
    ethereum::valence_account::approve_library(
        &rt,
        &eth_client,
        withdraw_acc_addr,
        *valence_vault.address(),
    );

    vault::query_vault_config(*valence_vault.address(), &rt, &eth_client);
    let vault_total_assets =
        vault::query_vault_total_assets(*valence_vault.address(), &rt, &eth_client);
    let vault_total_supply =
        vault::query_vault_total_supply(*valence_vault.address(), &rt, &eth_client);
    let user_vault_bal =
        vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user_acc);

    info!("vault total assets: {:?}", vault_total_assets._0);
    info!("vault total supply: {:?}", vault_total_supply._0);
    info!("user vault balance: {:?}", user_vault_bal._0);

    info!("Approving token for vault...");
    ethereum::mock_erc20::approve(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_admin_acc,
        *valence_vault.address(),
        U256::MAX,
    );

    let deposit_amount = U256::from(500_000);

    let vault_state = vault::query_vault_packed_values(*valence_vault.address(), &rt, &eth_client);
    info!("vault packed values: {:?}", vault_state);

    info!("User depositing {deposit_amount}USDC tokens to vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_user_acc,
        deposit_amount,
    )?;

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
    )
    .unwrap();

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

    info!("pausing the vault...");
    vault::pause(*valence_vault.address(), &rt, &eth_client)?;

    info!("attempting to deposit to paused vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_user_acc,
        deposit_amount,
    )?;

    info!("resuming the vault...");
    vault::unpause(*valence_vault.address(), &rt, &eth_client)?;

    info!("attempting to deposit to active vault...");
    vault::deposit_to_vault(
        &rt,
        &eth_client,
        *valence_vault.address(),
        eth_user_acc,
        deposit_amount,
    )?;

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
    )?;

    info!("minting some USDC for admin...");
    ethereum::mock_erc20::mint(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_admin_acc,
        deposit_amount * U256::from(5),
    );

    info!("transferring USDC from admin to withdraw account...");
    ethereum::mock_erc20::transfer(
        &rt,
        &eth_client,
        usdc_token_address,
        eth_admin_acc,
        withdraw_acc_addr,
        deposit_amount * U256::from(5),
    );

    async_run!(rt, {
        let withdraw_account = BaseAccount::new(withdraw_acc_addr, &eth_rp);

        let approve_calldata = usdc_token
            .approve(*valence_vault.address(), U256::MAX)
            .calldata()
            .clone();

        eth_client
            .execute_tx(
                withdraw_account
                    .execute(usdc_token_address, U256::from(0), approve_calldata)
                    .into_transaction_request(),
            )
            .await
            .unwrap();

        let allowance = eth_client
            .query(usdc_token.allowance(withdraw_acc_addr, *valence_vault.address()))
            .await
            .unwrap();

        info!("Withdraw account has approved vault for: {}", allowance._0);

        info!("asserting that vault is approved by the withdraw account...");

        let withdraw_account = BaseAccount::new(withdraw_acc_addr, &eth_rp);

        let is_approved = eth_client
            .query(withdraw_account.approvedLibraries(*valence_vault.address()))
            .await
            .unwrap();

        info!(
            "vault approved as library for withdraw account: {}",
            is_approved._0
        );
        let bal = eth_client
            .query(usdc_token.balanceOf(withdraw_acc_addr))
            .await
            .unwrap()
            ._0;
        info!("ETH WITHDRAW ACC USDC BAL\t: {bal}");
    });

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
    )
    .unwrap();

    info!("User initiates shares redeemal...");

    let user_shares =
        vault::query_vault_balance_of(*valence_vault.address(), &rt, &eth_client, eth_user_acc)._0;

    vault::redeem(
        *valence_vault.address(),
        &rt,
        &eth_client,
        eth_user_acc,
        user_shares,
        10_000,
        true,
    )?;

    let has_active_withdraw =
        vault::addr_has_active_withdraw(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    info!("user active withdraws: {:?}", has_active_withdraw._0);

    let user_withdraw_request =
        vault::addr_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    info!("user active withdraw request: {:?}", user_withdraw_request);

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
    )
    .unwrap();

    sleep(Duration::from_secs(3));

    info!("user attempts to finalize the withdrawal...");
    vault::complete_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc)?;

    let has_active_withdraw =
        vault::addr_has_active_withdraw(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    info!("user active withdraws: {:?}", has_active_withdraw._0);

    let user_withdraw_request =
        vault::addr_withdraw_request(*valence_vault.address(), &rt, &eth_client, eth_user_acc);
    info!("user active withdraw request: {:?}", user_withdraw_request);

    log_eth_balances(
        &eth_client,
        &rt,
        valence_vault.address(),
        &usdc_token_address,
        &deposit_acc_addr,
        &withdraw_acc_addr,
        &eth_user_acc,
    )
    .unwrap();

    Ok(())
}

fn log_eth_balances(
    eth_client: &EthereumClient,
    rt: &tokio::runtime::Runtime,
    vault_addr: &Address,
    vault_deposit_token: &Address,
    deposit_acc_addr: &Address,
    withdraw_acc_addr: &Address,
    depositor_addr: &Address,
) -> Result<(), Box<dyn Error>> {
    async_run!(rt, {
        let eth_rp = eth_client.get_request_provider().await.unwrap();

        let usdc_token = MockERC20::new(*vault_deposit_token, &eth_rp);
        let valence_vault = ValenceVault::new(*vault_addr, &eth_rp);

        let (
            depositor_usdc_bal,
            depositor_vault_bal,
            withdraw_acc_usdc_bal,
            deposit_acc_usdc_bal,
            vault_total_supply,
        ) = tokio::join!(
            eth_client.query(usdc_token.balanceOf(*depositor_addr)),
            eth_client.query(valence_vault.balanceOf(*depositor_addr)),
            eth_client.query(usdc_token.balanceOf(*withdraw_acc_addr)),
            eth_client.query(usdc_token.balanceOf(*deposit_acc_addr)),
            eth_client.query(valence_vault.totalSupply()),
        );

        let depositor_usdc_bal = depositor_usdc_bal.unwrap()._0;
        let depositor_vault_bal = depositor_vault_bal.unwrap()._0;
        let withdraw_acc_usdc_bal = withdraw_acc_usdc_bal.unwrap()._0;
        let deposit_acc_usdc_bal = deposit_acc_usdc_bal.unwrap()._0;
        let vault_total_supply = vault_total_supply.unwrap()._0;

        info!("USER SHARES\t\t: {depositor_vault_bal}");
        info!("USER USDC\t\t: {depositor_usdc_bal}");
        info!("WITHDRAW ACC USDC\t: {withdraw_acc_usdc_bal}");
        info!("DEPOSIT ACC USDC\t: {deposit_acc_usdc_bal}");
        info!("VAULT TOTAL SUPPLY\t: {vault_total_supply}");
    });

    Ok(())
}

fn setup_valence_encoder_broker(
    test_ctx: &mut TestContext,
    evm_encoder: String,
) -> Result<String, Box<dyn Error>> {
    let current_dir = env::current_dir()?;
    let encoder_broker_path = format!(
        "{}/artifacts/valence_encoder_broker.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&encoder_broker_path)?;

    let code_id_encoder_broker = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_encoder_broker")
        .unwrap();

    let encoder_broker = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_encoder_broker,
        &serde_json::to_string(&valence_encoder_broker::msg::InstantiateMsg {
            encoders: HashMap::from([(EVM_ENCODER_NAMESPACE.to_string(), evm_encoder)]),
            owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        })
        .unwrap(),
        "encoder_broker",
        None,
        "",
    )
    .unwrap()
    .address;

    info!("EVM broker: {encoder_broker}");

    Ok(encoder_broker)
}

fn setup_valence_evm_encoder_v1(test_ctx: &mut TestContext) -> Result<String, Box<dyn Error>> {
    let current_dir = env::current_dir()?;

    let evm_encoder_path = format!(
        "{}/artifacts/valence_evm_encoder_v1.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&evm_encoder_path)?;

    let code_id_evm_encoder = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get("valence_evm_encoder_v1")
        .unwrap();

    let evm_encoder = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id_evm_encoder,
        &serde_json::to_string(&Empty {}).unwrap(),
        "evm_encoder",
        None,
        "",
    )
    .unwrap()
    .address;

    info!("EVM encoder: {evm_encoder}");

    Ok(evm_encoder)
}

#[allow(clippy::too_many_arguments)]
fn test_neutron_side_flow(
    test_ctx: &mut TestContext,
    deposit_acc_addr: &str,
    position_acc_addr: &str,
    withdraw_acc_addr: &str,
    denom_1: &str,
    denom_2: &str,
    authorizations_addr: &str,
    ntrn_processor_addr: &str,
    encoder_broker: &str,
    ntrn_mailbox: &str,
    lite_processor_address: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Adding EVM external domain to Authorization contract");

    let authorization_exec_env_info =
        valence_authorization_utils::msg::ExecutionEnvironmentInfo::Evm(
            EncoderInfo {
                broker_address: encoder_broker.to_string(),
                encoder_version: EVM_ENCODER_NAMESPACE.to_string(),
            },
            EvmBridgeInfo::Hyperlane(HyperlaneConnectorInfo {
                mailbox: ntrn_mailbox.to_string(),
                domain_id: ETHEREUM_HYPERLANE_DOMAIN,
            }),
        );

    let external_domain_info = ExternalDomainInfo {
        name: ETHEREUM_CHAIN_NAME.to_string(),
        execution_environment: authorization_exec_env_info,
        processor: lite_processor_address.to_string(),
    };

    let add_external_evm_domain_msg =
        valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
            PermissionedMsg::AddExternalDomains {
                external_domains: vec![external_domain_info],
            },
        );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorizations_addr,
        DEFAULT_KEY,
        &serde_json::to_string(&add_external_evm_domain_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    info!("funding the input account...");
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        deposit_acc_addr,
        &[
            cosmwasm_std_old::Coin {
                denom: denom_2.to_string(),
                amount: 1_000_000u128.into(),
            },
            cosmwasm_std_old::Coin {
                denom: denom_1.to_string(),
                amount: 1_200_000u128.into(),
            },
        ],
        &cosmwasm_std_old::Coin {
            denom: denom_1.to_string(),
            amount: 1_000_000u128.into(),
        },
    )?;

    std::thread::sleep(Duration::from_secs(3));

    log_neutron_acc_balances(
        test_ctx,
        deposit_acc_addr,
        position_acc_addr,
        withdraw_acc_addr,
    );

    let lp_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideDoubleSidedLiquidity {
                    expected_pool_ratio_range: None,
                },
            ),
        )?),
    };
    let provide_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL.to_string(),
            messages: vec![lp_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorizations_addr,
        DEFAULT_KEY,
        &serde_json::to_string(&provide_liquidity_msg)?,
        GAS_FLAGS,
    )?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    tick_processor(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        ntrn_processor_addr,
        GAS_FLAGS,
    );
    std::thread::sleep(std::time::Duration::from_secs(2));

    log_neutron_acc_balances(
        test_ctx,
        deposit_acc_addr,
        position_acc_addr,
        withdraw_acc_addr,
    );

    info!("pushing withdraw liquidity message to processor...");
    let lw_message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {
                    expected_pool_ratio_range: None,
                },
            ),
        )?),
    };
    let withdraw_liquidity_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL.to_string(),
            messages: vec![lw_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        authorizations_addr,
        DEFAULT_KEY,
        &serde_json::to_string(&withdraw_liquidity_msg)?,
        GAS_FLAGS,
    )?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("ticking processor to withdraw liquidity");
    tick_processor(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        ntrn_processor_addr,
        GAS_FLAGS,
    );
    std::thread::sleep(std::time::Duration::from_secs(2));

    log_neutron_acc_balances(
        test_ctx,
        deposit_acc_addr,
        position_acc_addr,
        withdraw_acc_addr,
    );

    Ok(())
}

fn log_neutron_acc_balances(
    test_ctx: &mut TestContext,
    deposit_acc: &str,
    position_acc: &str,
    withdraw_acc: &str,
) {
    let deposit_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        deposit_acc,
    );
    let position_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        position_acc,
    );
    let withdraw_acc_bal = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        withdraw_acc,
    );
    info!("DEPOSIT ACC BAL\t: {:?}", deposit_acc_bal);
    info!("POSITION ACC BAL\t: {:?}", position_acc_bal);
    info!("WITHDRAW ACC BAL\t: {:?}", withdraw_acc_bal);
}

fn deploy_astroport_contracts(
    test_ctx: &mut TestContext,
) -> Result<(u64, u64, u64, u64), Box<dyn Error>> {
    info!("Uploading astroport contracts...");
    let current_dir = env::current_dir()?;
    let astroport_contracts_path = format!("{}/{}", current_dir.display(), ASTROPORT_PATH);

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(&astroport_contracts_path, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)?;

    // Set up the astroport factory and the pool
    let astroport_factory_code_id = test_ctx
        .get_contract()
        .contract("astroport_factory")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_pair_concentrated_code_id = test_ctx
        .get_contract()
        .contract("astroport_pair_concentrated")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_token_code_id = test_ctx
        .get_contract()
        .contract("astroport_token")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_coin_registry_code_id = test_ctx
        .get_contract()
        .contract("astroport_native_coin_registry")
        .get_cw()
        .code_id
        .unwrap();

    Ok((
        astroport_factory_code_id,
        astroport_pair_concentrated_code_id,
        astroport_token_code_id,
        astroport_coin_registry_code_id,
    ))
}

fn setup_astroport_cl_pool(
    test_ctx: &mut TestContext,
    pair_code_id: u64,
    token_code_id: u64,
    factory_code_id: u64,
    native_coin_registry_code_id: u64,
    denom: String,
) -> Result<(String, String), Box<dyn Error>> {
    info!("Instantiating astroport native coin registry...");
    let coin_registry_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        native_coin_registry_code_id,
        &serde_json::to_string(&NativeCoinRegistryInstantiateMsg {
            owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        })
        .unwrap(),
        "astro_native_coin_registry",
        None,
        "",
    )
    .unwrap();

    info!(
        "Astroport native coin registry address: {}",
        coin_registry_contract.address.clone()
    );

    info!("whitelisting coin registry native coins...");
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &coin_registry_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(&NativeCoinRegistryExecuteMsg::Add {
            native_coins: vec![(NEUTRON_CHAIN_DENOM.to_string(), 6), (denom.to_string(), 6)],
        })
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Instantiating astroport factory...");
    let astroport_factory_instantiate_msg = FactoryInstantiateMsg {
        pair_configs: vec![PairConfig {
            code_id: pair_code_id,
            pair_type: PairType::Custom(ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string()),
            total_fee_bps: 0u16,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: false,
            permissioned: false,
        }],
        token_code_id,
        fee_address: None,
        generator_address: None,
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        whitelist_code_id: 234, // This is not needed anymore but still part of API
        coin_registry_address: coin_registry_contract.address.to_string(),
        tracker_config: None,
    };

    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        factory_code_id,
        &serde_json::to_string(&astroport_factory_instantiate_msg).unwrap(),
        "astroport_factory",
        None,
        "",
    )
    .unwrap();

    info!(
        "Astroport factory address: {}",
        factory_contract.address.clone()
    );

    info!("Create the pool...");
    let pool_assets = vec![
        AssetInfo::NativeToken {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: denom.clone(),
        },
    ];

    let default_params = ConcentratedPoolParams {
        amp: Decimal::from_ratio(40u128, 1u128),
        gamma: Decimal::from_ratio(145u128, 1000000u128),
        mid_fee: Decimal::from_str("0.0026").unwrap(),
        out_fee: Decimal::from_str("0.0045").unwrap(),
        fee_gamma: Decimal::from_ratio(23u128, 100000u128),
        repeg_profit_threshold: Decimal::from_ratio(2u128, 1000000u128),
        min_price_scale_delta: Decimal::from_ratio(146u128, 1000000u128),
        price_scale: Decimal::one(),
        ma_half_time: 600,
        track_asset_balances: None,
        fee_share: None,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_astroport_utils::astroport_native_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: PairType::Custom(ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string()),
                asset_infos: pool_assets.clone(),
                init_params: Some(to_json_binary(&default_params).unwrap()),
            },
        )
        .unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let query_pool_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &factory_contract.address.clone(),
            &serde_json::to_string(&FactoryQueryMsg::Pair {
                asset_infos: pool_assets.clone(),
            })
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let pool_addr = query_pool_response["contract_addr"].as_str().unwrap();
    let lp_token = query_pool_response["liquidity_token"].as_str().unwrap();

    info!("Pool created successfully! Pool address: {pool_addr}, LP token: {lp_token}");
    let asset_a = coin(799_000_000, NEUTRON_CHAIN_DENOM);
    let asset_b = coin(999_000_000, denom.clone());
    let assets = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: asset_a.denom.to_string(),
            },
            amount: asset_a.amount,
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: asset_b.denom.to_string(),
            },
            amount: asset_b.amount,
        },
    ];

    let initial_lp_msg = ConcentratedLiquidityExecuteMsg::ProvideLiquidity {
        assets,
        slippage_tolerance: None,
        auto_stake: None,
        receiver: None,
        min_lp_to_receive: None,
    };

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        pool_addr,
        DEFAULT_KEY,
        &serde_json::to_string(&initial_lp_msg).unwrap(),
        &format!(
            "--amount {}{},{}{} --gas 1000000",
            asset_a.amount.u128(),
            asset_a.denom,
            asset_b.amount.u128(),
            asset_b.denom
        ),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    Ok((pool_addr.to_string(), lp_token.to_string()))
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
