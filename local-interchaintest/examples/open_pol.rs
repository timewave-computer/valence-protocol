use cosmwasm_std_old::Coin as BankCoin;
use rand::{distributions::Alphanumeric, Rng};
use std::{
    collections::{BTreeMap, HashSet},
    env,
    error::Error,
    thread::sleep,
    time::{Duration, SystemTime},
};

use cosmwasm_std::{Binary, Decimal, Timestamp, Uint128};
use cw_utils::Expiration;
use local_interchaintest::utils::{
    base_account::create_base_accounts,
    manager::{
        setup_manager, use_manager_init, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME,
        DETOKENIZER_NAME, FORWARDER_NAME, SPLITTER_NAME, TOKENIZER_NAME,
    },
    processor::tick_processor,
    ASTROPORT_PATH, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON, LOGS_FILE_PATH,
    NEUTRON_CONFIG_FILE, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate, contract_query},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};

use log::info;
use serde_json::Value;
use valence_astroport_lper::msg::{AssetData, LiquidityProviderConfig};
use valence_astroport_utils::astroport_native_lp_token::{
    Asset, AssetInfo, FactoryInstantiateMsg, FactoryQueryMsg, PairConfig, PairType,
};

use valence_authorization_utils::{
    authorization::{AuthorizationDuration, AuthorizationModeInfo, PermissionTypeInfo},
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_detokenizer_service::msg::DetokenizerConfig;
use valence_service_utils::{denoms::UncheckedDenom, msg::ValenceServiceQuery, ServiceAccountType};
use valence_splitter_service::msg::{UncheckedSplitAmount, UncheckedSplitConfig};
use valence_workflow_manager::{
    account::{AccountInfo, AccountType},
    service::{ServiceConfig, ServiceInfo},
    workflow_config_builder::WorkflowConfigBuilder,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    setup_manager(
        &mut test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            TOKENIZER_NAME,
            DETOKENIZER_NAME,
            SPLITTER_NAME,
            ASTROPORT_LPER_NAME,
            ASTROPORT_WITHDRAWER_NAME,
            FORWARDER_NAME,
        ],
    )?;

    let mut builder = WorkflowConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_workflow_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());
    let mut uploader = test_ctx.build_tx_upload_contracts();

    info!("Uploading astroport contracts...");
    let current_dir = env::current_dir()?;
    let astroport_contracts_path = format!("{}/{}", current_dir.display(), ASTROPORT_PATH);

    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(&astroport_contracts_path, LOCAL_CODE_ID_CACHE_PATH_NEUTRON)?;

    // Set up the astroport factory and the pool
    let astroport_factory_code_id = test_ctx
        .get_contract()
        .contract("astroport_factory_native")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_pair_native_code_id = test_ctx
        .get_contract()
        .contract("astroport_pair_native")
        .get_cw()
        .code_id
        .unwrap();

    let astroport_token_code_id = test_ctx
        .get_contract()
        .contract("astroport_token")
        .get_cw()
        .code_id
        .unwrap();

    info!("Instantiating astroport factory...");
    let astroport_factory_instantiate_msg = FactoryInstantiateMsg {
        pair_configs: vec![PairConfig {
            code_id: astroport_pair_native_code_id,
            pair_type: PairType::Xyk {},
            total_fee_bps: 0,
            maker_fee_bps: 0,
            is_disabled: false,
            is_generator_disabled: true,
            permissioned: false,
        }],
        token_code_id: astroport_token_code_id,
        fee_address: None,
        generator_address: None,
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        whitelist_code_id: 0, // This is not needed anymore but still part of API
        coin_registry_address: NEUTRON_CHAIN_ADMIN_ADDR.to_string(), // Passing any address here is fine as long as it's a valid one
        tracker_config: None,
    };

    let factory_contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        astroport_factory_code_id,
        &serde_json::to_string(&astroport_factory_instantiate_msg).unwrap(),
        "processor",
        None,
        "",
    )
    .unwrap();
    info!(
        "Astroport factory address: {}",
        factory_contract.address.clone()
    );

    // TODO(REMOVE): This is a temporary solution to mint a meme coin for testing purposes
    let token1_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token1_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token1 = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token1_subdenom.clone())
        .get();

    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(1_000_000_000_000_000)
        .with_denom(&token1)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Find the Meme coin that we minted via the front end
    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &NEUTRON_CHAIN_ADMIN_ADDR,
    );
    info!("Neutron chain admin balance: {:?}", balance);
    let meme_coin = balance
        .iter()
        .find(|coin| {
            coin.denom.contains("factory") && coin.denom.contains(token1_subdenom.as_str())
        })
        .expect("Meme coin not found");
    info!("Meme coin: {:?}", meme_coin);

    // Create the pool
    let pool_assets = vec![
        AssetInfo::NativeToken {
            denom: NTRN_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: meme_coin.denom.clone(),
        },
    ];
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &factory_contract.address,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_astroport_utils::astroport_native_lp_token::FactoryExecuteMsg::CreatePair {
                pair_type: PairType::Xyk {},
                asset_infos: pool_assets.clone(),
                init_params: None,
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
    info!(
        "Pool created successfully! Pool address: {}, LP token: {}",
        pool_addr, lp_token
    );

    info!("Provide some initial liquidity to the pool...");
    // We'll provide with ratio 1:2
    let ntrn_deposit = 250000000;
    let token_deposit = 500000000;
    let provide_liquidity_msg =
        valence_astroport_utils::astroport_native_lp_token::ExecuteMsg::ProvideLiquidity {
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: NTRN_DENOM.to_string(),
                    },
                    amount: Uint128::new(ntrn_deposit),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: meme_coin.denom.clone(),
                    },
                    amount: Uint128::new(token_deposit),
                },
            ],
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
        &serde_json::to_string(&provide_liquidity_msg).unwrap(),
        &format!(
            "--amount {}{},{}{} {}",
            token_deposit,
            meme_coin.denom.clone(),
            ntrn_deposit,
            NTRN_DENOM,
            GAS_FLAGS
        ),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let account_1 = builder.add_account(AccountInfo::new(
        "test_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_2 = builder.add_account(AccountInfo::new(
        "test_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_3 = builder.add_account(AccountInfo::new(
        "test_3".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_4 = builder.add_account(AccountInfo::new(
        "test_4".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_5 = builder.add_account(AccountInfo::new(
        "test_5".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let mut price_map = BTreeMap::new();
    price_map.insert(NTRN_DENOM.to_string(), Uint128::one());
    let tokenizer_service = builder.add_service(ServiceInfo {
        name: "test_tokenizer".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceTokenizerService(
            valence_tokenizer_service::msg::ServiceConfig {
                output_addr: account_1.clone(),
                input_denoms: price_map,
            },
        ),
        addr: None,
    });

    let lper_service = builder.add_service(ServiceInfo {
        name: "test_lper".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceAstroportLper(valence_astroport_lper::msg::ServiceConfig {
            input_addr: account_1.clone(),
            output_addr: account_2.clone(),
            pool_addr: pool_addr.to_string(),
            lp_config: LiquidityProviderConfig {
                pool_type: valence_astroport_lper::msg::PoolType::NativeLpToken(PairType::Xyk {}),
                asset_data: AssetData {
                    asset1: NTRN_DENOM.to_string(),
                    asset2: meme_coin.denom.clone(),
                },
                slippage_tolerance: None,
            },
        }),
        addr: None,
    });

    builder.add_link(&lper_service, vec![&account_1], vec![&account_2]);

    let withdrawer_service = builder.add_service(ServiceInfo {
        name: "test_withdrawer".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceAstroportWithdrawer(
            valence_astroport_withdrawer::msg::ServiceConfig {
                input_addr: account_2.clone(),
                output_addr: account_3.clone(),
                pool_addr: pool_addr.to_string(),
                withdrawer_config: valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
                    pool_type: valence_astroport_withdrawer::msg::PoolType::NativeLpToken,
                },
            },
        ),
        addr: None,
    });
    builder.add_link(&withdrawer_service, vec![&account_2], vec![&account_3]);

    let splitter_service = builder.add_service(ServiceInfo {
        name: "test_splitter".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceSplitterService(
            valence_splitter_service::msg::ServiceConfig {
                input_addr: account_3.clone(),
                splits: vec![
                    UncheckedSplitConfig {
                        denom: UncheckedDenom::Native(meme_coin.denom.clone()),
                        account: account_4.clone(),
                        amount: UncheckedSplitAmount::FixedRatio(Decimal::percent(95)),
                    },
                    UncheckedSplitConfig {
                        denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                        account: account_5.clone(),
                        amount: UncheckedSplitAmount::FixedRatio(Decimal::percent(100)),
                    },
                    UncheckedSplitConfig {
                        denom: UncheckedDenom::Native(meme_coin.denom.clone()),
                        account: account_5.clone(),
                        amount: UncheckedSplitAmount::FixedRatio(Decimal::percent(5)),
                    },
                ],
            },
        ),
        addr: None,
    });
    builder.add_link(
        &splitter_service,
        vec![&account_3],
        vec![&account_4, &account_5],
    );

    let detokenizer_service = builder.add_service(ServiceInfo {
        name: "test_detokenizer".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceDetokenizerService(
            valence_detokenizer_service::msg::ServiceConfig {
                input_addr: account_5.clone(),
                detokenizer_config: DetokenizerConfig {
                    input_addr: account_3.clone(),
                    voucher_denom: "dumdum".to_string(), // Need to update it
                    redeemable_denoms: HashSet::from_iter(vec![
                        meme_coin.denom.clone(),
                        NTRN_DENOM.to_string(),
                    ]),
                },
            },
        ),
        addr: None,
    });

    let dummy_vec: Vec<&ServiceAccountType> = vec![];
    builder.add_link(&detokenizer_service, vec![&account_5.clone()], dummy_vec);
    let dummy_vec: Vec<&ServiceAccountType> = vec![];
    builder.add_link(&tokenizer_service, dummy_vec, vec![&account_1.clone()]);

    let now = SystemTime::now();
    let time_now = now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();

    let authorization_1 = AuthorizationBuilder::new()
        .with_label("tokenize")
        .with_duration(AuthorizationDuration::Seconds(120))
        .with_max_concurrent_executions(10)
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(ServiceAccountType::ServiceId(0))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();
    builder.add_authorization(authorization_1);

    let authorization_2 = AuthorizationBuilder::new()
        .with_label("provide_liquidity")
        .with_not_before(Expiration::AtTime(Timestamp::from_seconds(time_now + 120)))
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(ServiceAccountType::ServiceId(1))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    builder.add_authorization(authorization_2);

    let authorization_3 = AuthorizationBuilder::new()
        .with_label("withdraw_and_split")
        .with_not_before(Expiration::AtTime(Timestamp::from_seconds(time_now + 150)))
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(ServiceAccountType::ServiceId(2))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(ServiceAccountType::ServiceId(3))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    builder.add_authorization(authorization_3);

    let authorization_4 = AuthorizationBuilder::new()
        .with_label("detokenize")
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(ServiceAccountType::ServiceId(4))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "process_action".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    builder.add_authorization(authorization_4);

    let authorization_5 = AuthorizationBuilder::new()
        .with_label("update_config")
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![NEUTRON_CHAIN_ADMIN_ADDR.to_string()]),
        ))
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(
                    AtomicActionBuilder::new()
                        .with_contract_address(ServiceAccountType::ServiceId(4))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "update_config".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build();

    builder.add_authorization(authorization_5);

    let mut built_config = builder.build();
    use_manager_init(&mut built_config)?;

    let current_dir: std::path::PathBuf = env::current_dir()?;
    let base_account_contract_path = format!(
        "{}/artifacts/valence_base_account.wasm",
        current_dir.display()
    );
    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&base_account_contract_path)?;

    let base_account_code_id = test_ctx
        .get_contract()
        .src(NEUTRON_CHAIN_NAME)
        .contract("valence_base_account")
        .get_cw()
        .code_id
        .unwrap();

    let base_accounts = create_base_accounts(
        &mut test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        base_account_code_id,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![
            built_config.get_service(0).unwrap().addr.unwrap(),
            built_config.get_service(4).unwrap().addr.unwrap(),
        ],
        5,
    );

    // Fund the base accounts
    for acc in base_accounts.clone() {
        bank::send(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            DEFAULT_KEY,
            &acc,
            &[BankCoin {
                denom: NTRN_DENOM.to_string(),
                amount: 1_000_000_000u128.into(),
            }],
            &BankCoin {
                denom: NTRN_DENOM.to_string(),
                amount: cosmwasm_std_old::Uint128::new(5000),
            },
        )
        .unwrap();

        sleep(Duration::from_secs(3));
    }

    // Get authorization and processor contract
    let authorization_contract_address = built_config.authorization_data.authorization_addr.clone();
    let processor_contract_address = built_config
        .get_processor_addr(&neutron_domain.to_string())
        .unwrap();

    // Update the config to use the voucher denom
    let address_account_3 = built_config
        .get_account(account_3)
        .unwrap()
        .addr
        .clone()
        .unwrap()
        .clone();

    // Need to get the tokenizer contract because the voucher denom uses that
    let tokenizer_contract_address = built_config.get_service(0).unwrap().addr.unwrap().clone();
    let voucher_denom = format!("factory/{}/tokenizer", tokenizer_contract_address,);

    let binary = Binary::from(
        serde_json::to_vec(&valence_service_utils::msg::ExecuteMsg::<
            (),
            valence_detokenizer_service::msg::ServiceConfigUpdate,
        >::UpdateConfig {
            new_config: valence_detokenizer_service::msg::ServiceConfigUpdate {
                input_addr: None,
                detokenizer_config: Some(DetokenizerConfig {
                    input_addr: ServiceAccountType::Addr(address_account_3),
                    voucher_denom,
                    redeemable_denoms: HashSet::from_iter(vec![
                        meme_coin.denom.clone(),
                        NTRN_DENOM.to_string(),
                    ]),
                }),
            },
        })
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "update_config".to_string(),
            messages: vec![message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&send_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the authorization contract!");
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor and executing config update...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    // Get detokenizer address
    let detokenizer_addr = built_config.get_service(4).unwrap().addr.unwrap().clone();
    // Query to check that it has been updated
    let data = contract_query(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &detokenizer_addr,
        &serde_json::to_string(&ValenceServiceQuery::GetServiceConfig {}).unwrap(),
    );
    info!("Detokenizer config: {:?}", data);

    // Get the account 1 to deposit the meme coin there
    let address_account_1 = built_config
        .get_account(account_1)
        .unwrap()
        .addr
        .clone()
        .unwrap()
        .clone();

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &address_account_1,
        &[BankCoin {
            denom: token1.to_string(),
            amount: 1_000_000_000u128.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();

    info!("tokenize for users");
    let tokenize_execute_messages: Vec<valence_authorization_utils::msg::ExecuteMsg> =
        base_accounts
            .clone()
            .into_iter()
            .map(|acc| {
                valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(
                    valence_tokenizer_service::msg::ActionMsgs::Tokenize { sender: acc },
                )
            })
            .map(|msg| Binary::from(serde_json::to_vec(&msg).unwrap()))
            .map(|binary| ProcessorMessage::CosmwasmExecuteMsg { msg: binary })
            .map(|processor_msg| {
                valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                    valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
                        label: "tokenize".to_string(),
                        messages: vec![processor_msg],
                        ttl: None,
                    },
                )
            })
            .collect();

    for msg in tokenize_execute_messages {
        info!("queuing the tokenize msg");
        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &authorization_contract_address,
            DEFAULT_KEY,
            &serde_json::to_string(&msg).unwrap(),
            GAS_FLAGS,
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));

        info!("start the tickin");
        tick_processor(
            &mut test_ctx,
            NEUTRON_CHAIN_NAME,
            DEFAULT_KEY,
            &processor_contract_address,
        );
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    info!("checking balance");
    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &address_account_1,
    );
    info!("balance of acc 1 : {:?}", balance);

    info!("Send the messages to the authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(
                valence_astroport_lper::msg::ActionMsgs::ProvideDoubleSidedLiquidity {
                    expected_pool_ratio_range: None,
                },
            ),
        )
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "provide_liquidity".to_string(),
            messages: vec![message],
            ttl: None,
        },
    );

    // Wait until time_now + 120
    let now = SystemTime::now();
    let current_time = now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
    let time_to_wait = 125u64.saturating_sub(current_time - time_now);
    std::thread::sleep(std::time::Duration::from_secs(time_to_wait));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&send_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    info!("Messages sent to the authorization contract!");
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor and executing LP...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    let address_account_2 = built_config
        .get_account(account_2)
        .unwrap()
        .addr
        .clone()
        .unwrap()
        .clone();

    info!("checking balance");
    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &address_account_2,
    );
    info!("balance of acc 2 : {:?}", balance);

    // WITHDRAW LIQUIDITY & SPLIT

    info!("Send the messages to the authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(
                valence_astroport_withdrawer::msg::ActionMsgs::WithdrawLiquidity {},
            ),
        )
        .unwrap(),
    );
    let withdraw_message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let binary = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(
                valence_splitter_service::msg::ActionMsgs::Split {},
            ),
        )
        .unwrap(),
    );
    let split_message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let withdraw_n_split_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "withdraw_and_split".to_string(),
            messages: vec![withdraw_message, split_message],
            ttl: None,
        },
    );

    // Wait until time_now + 150
    let now = SystemTime::now();
    let current_time = now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
    let time_to_wait = 155u64.saturating_sub(current_time - time_now);
    std::thread::sleep(std::time::Duration::from_secs(time_to_wait));

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&withdraw_n_split_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor and executing withdraw n split...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    let address_account_4 = built_config
        .get_account(account_4)
        .unwrap()
        .addr
        .clone()
        .unwrap()
        .clone();

    let address_account_5 = built_config
        .get_account(account_5)
        .unwrap()
        .addr
        .clone()
        .unwrap()
        .clone();

    info!("checking balances");
    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &address_account_4,
    );
    info!("balance of acc 4 : {:?}", balance);
    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &address_account_5,
    );
    info!("balance of acc 5 : {:?}", balance);

    // DETOKENIZE
    info!("Send the messages to the authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(
                valence_detokenizer_service::msg::ActionMsgs::Detokenize {
                    addresses: HashSet::from_iter(base_accounts.clone()),
                },
            ),
        )
        .unwrap(),
    );
    let detokenize_message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let detokenize_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "detokenize".to_string(),
            messages: vec![detokenize_message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        DEFAULT_KEY,
        &serde_json::to_string(&detokenize_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Ticking processor and executing detokenize...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    info!("checking balances of all base accounts");
    for base_account in base_accounts {
        let balance = bank::get_balance(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &base_account,
        );
        info!("balance of acc : {:?} : {:?}", base_account, balance);
    }

    info!("SUCCESS!");
    Ok(())
}
