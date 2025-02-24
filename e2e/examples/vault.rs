use std::{collections::HashMap, env, error::Error, str::FromStr, time::Duration};

use alloy::primitives::Address;
use cosmwasm_std::{coin, to_json_binary, Addr, Binary, Decimal, Empty};
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

use log::info;
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
use valence_e2e::utils::{
    ethereum::set_up_anvil_container,
    hyperlane::{
        bech32_to_evm_bytes32, set_up_cw_hyperlane_contracts, set_up_eth_hyperlane_contracts,
        set_up_hyperlane, HyperlaneContracts,
    },
    manager::{setup_manager, use_manager_init, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME},
    processor::tick_processor,
    solidity_contracts::{BaseAccount, LiteProcessor, ValenceVault},
    ASTROPORT_PATH, DEFAULT_ANVIL_RPC_ENDPOINT, ETHEREUM_CHAIN_NAME, ETHEREUM_HYPERLANE_DOMAIN,
    GAS_FLAGS, HYPERLANE_RELAYER_NEUTRON_ADDRESS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH,
    NEUTRON_CONFIG_FILE, NEUTRON_HYPERLANE_DOMAIN, VALENCE_ARTIFACTS_PATH,
};
use valence_library_utils::liquidity_utils::AssetData;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::LibraryInfo,
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

const EVM_ENCODER_NAMESPACE: &str = "evm_encoder_v1";

pub fn my_evm_vault_program(
    ntrn_domain: valence_program_manager::domain::Domain,
    asset_1: &str,
    asset_2: &str,
    pool_addr: &str,
) -> Result<ProgramConfig, Box<dyn Error>> {
    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());

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
        "concentrated".to_string(),
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
        .with_label("provide_liquidity")
        .with_subroutine(astro_lper_subroutine)
        .build();
    let astro_lwer_authorization = AuthorizationBuilder::new()
        .with_label("withdraw_liquidity")
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

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    let (eth_hyperlane_contracts, ntrn_hyperlane_contracts) =
        hyperlane_plumbing(&mut test_ctx, &eth)?;

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

    // setup astroport
    let (
        astroport_factory_code_id,
        astroport_pair_concentrated_code_id,
        astroport_token_code_id,
        astroport_coin_registry_code_id,
    ) = deploy_astroport_contracts(&mut test_ctx)?;

    let (pool_addr, lp_token) = setup_astroport_cl_pool(
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

    let mut program_config =
        my_evm_vault_program(ntrn_domain.clone(), NEUTRON_CHAIN_DENOM, &token, &pool_addr)?;

    info!("initializing manager...");
    use_manager_init(&mut program_config)?;

    let deposit_acc_addr = program_config.get_account(0u64)?.addr.clone().unwrap();
    let position_acc_addr = program_config.get_account(1u64)?.addr.clone().unwrap();
    let withdraw_acc_addr = program_config.get_account(2u64)?.addr.clone().unwrap();

    info!("DEPOSIT ACCOUNT\t: {deposit_acc_addr}");
    info!("POSITION ACCOUNT\t: {position_acc_addr}");
    info!("WITHDRAW ACCOUNT\t: {withdraw_acc_addr}");

    let authorization_contract_address =
        program_config.authorization_data.authorization_addr.clone();

    let ntrn_processor_contract_address = program_config
        .get_processor_addr(&ntrn_domain.to_string())
        .unwrap();

    info!("authorization contract address: {authorization_contract_address}");
    info!("ntrn processor contract address: {ntrn_processor_contract_address}");

    // setup eth side:
    // 0. encoders
    // 1. lite processor
    // 2. base accounts
    // 3. vault

    info!("Setting up encoders ...");
    let evm_encoder = setup_valence_evm_encoder_v1(&mut test_ctx)?;
    let encoder_broker = setup_valence_encoder_broker(&mut test_ctx, evm_encoder.to_string())?;

    info!("Setting up Lite Processor on Ethereum");
    let accounts = eth.get_accounts_addresses()?;

    let tx = LiteProcessor::deploy_builder(
        &eth.provider,
        bech32_to_evm_bytes32(&authorization_contract_address)?,
        Address::from_str(&eth_hyperlane_contracts.mailbox)?,
        NEUTRON_HYPERLANE_DOMAIN,
        vec![accounts[0]],
    )
    .into_transaction_request()
    .from(accounts[0]);

    let lite_processor_address = eth.send_transaction(tx)?.contract_address.unwrap();
    info!("Lite Processor deployed at: {}", lite_processor_address);

    info!("Adding EVM external domain to Authorization contract");
    let add_external_evm_domain_msg =
        valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
            PermissionedMsg::AddExternalDomains {
                external_domains: vec![ExternalDomainInfo {
                    name: ETHEREUM_CHAIN_NAME.to_string(),
                    execution_environment:
                        valence_authorization_utils::msg::ExecutionEnvironmentInfo::Evm(
                            EncoderInfo {
                                broker_address: evm_encoder.clone(),
                                encoder_version: EVM_ENCODER_NAMESPACE.to_string(),
                            },
                            EvmBridgeInfo::Hyperlane(HyperlaneConnectorInfo {
                                mailbox: ntrn_hyperlane_contracts.mailbox.to_string(),
                                domain_id: ETHEREUM_HYPERLANE_DOMAIN,
                            }),
                        ),
                    processor: lite_processor_address.to_string(),
                }],
            },
        );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&add_external_evm_domain_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(3));

    // Let's create two Valence Base Accounts on Ethereum to test the processor with libraries (in this case the forwarder)
    info!("Deploying base accounts on Ethereum...");
    let base_account_tx = BaseAccount::deploy_builder(&eth.provider, accounts[0], vec![])
        .into_transaction_request()
        .from(accounts[0]);

    let eth_deposit_acc = eth
        .send_transaction(base_account_tx.clone())?
        .contract_address
        .unwrap();
    let eth_withdraw_acc = eth
        .send_transaction(base_account_tx.clone())?
        .contract_address
        .unwrap();
    info!("ETH deposit acc: {:?}", eth_deposit_acc);
    info!("ETH withdraw acc: {:?}", eth_withdraw_acc);

    // info!("Deploying Valence Vault on Ethereum...");
    // let vault_tx = ValenceVault::deploy_builder(
    //     &eth.provider,
    //     accounts[0],
    //     config,
    //     underlying,
    //     vaultTokenName,
    //     vaultTokenSymbol,
    //     startingRate,
    // );

    // test the neutron side flow
    // test_neutron_side_flow(
    //     &mut test_ctx,
    //     &deposit_acc_addr,
    //     &position_acc_addr,
    //     &withdraw_acc_addr,
    //     NEUTRON_CHAIN_DENOM,
    //     &token,
    //     &authorization_contract_address,
    //     &ntrn_processor_contract_address,
    // )?;

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
    .unwrap();
    info!(
        "Encoders set up successfully! Broker address: {}",
        encoder_broker.address
    );

    Ok(encoder_broker.address)
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
    .unwrap();

    info!("EVM encoder: {:?}", evm_encoder.address);

    Ok(evm_encoder.address)
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
) -> Result<(), Box<dyn Error>> {
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
            label: "provide_liquidity".to_string(),
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
            label: "withdraw_liquidity".to_string(),
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
            pair_type: PairType::Custom("concentrated".to_string()),
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
                pair_type: PairType::Custom("concentrated".to_string()),
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
