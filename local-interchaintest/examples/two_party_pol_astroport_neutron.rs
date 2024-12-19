use std::{env, error::Error};

use cosmwasm_std::{Binary, Decimal, Uint128};
use cosmwasm_std_old::Coin;
use local_interchaintest::utils::{
    manager::{
        setup_manager, use_manager_init, ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME,
        FORWARDER_NAME, REVERSE_SPLITTER_NAME, SPLITTER_NAME,
    },
    processor::tick_processor,
    ASTROPORT_LP_SUBDENOM, ASTROPORT_PATH, GAS_FLAGS, LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
    LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, NEUTRON_USER_ADDRESS_1, NTRN_DENOM, USER_KEY_1,
    VALENCE_ARTIFACTS_PATH,
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
use rand::{distributions::Alphanumeric, Rng};
use serde_json::Value;
use valence_astroport_lper::msg::{AssetData, LiquidityProviderConfig};
use valence_astroport_utils::astroport_native_lp_token::{
    Asset, AssetInfo, FactoryInstantiateMsg, FactoryQueryMsg, PairConfig, PairType,
};
use valence_authorization_utils::{
    authorization::{AuthorizationModeInfo, PermissionTypeInfo},
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_forwarder_library::msg::{ForwardingConstraints, UncheckedForwardingConfig};
use valence_library_utils::{denoms::UncheckedDenom, LibraryAccountType};
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config::{Link, ProgramConfig},
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

    info!("Create the pool...");
    let pool_assets = vec![
        AssetInfo::NativeToken {
            denom: NTRN_DENOM.to_string(),
        },
        AssetInfo::NativeToken {
            denom: token.clone(),
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
                        denom: token.clone(),
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
            token.clone(),
            ntrn_deposit,
            NTRN_DENOM,
            GAS_FLAGS
        ),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Set up the program manager...");
    let mut program_config = ProgramConfig {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        ..Default::default()
    };
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    // We will need 10 base accounts
    for i in 1..11 {
        program_config.accounts.insert(
            i,
            AccountInfo {
                name: format!("base_account_{}", i),
                ty: AccountType::Base { admin: None },
                domain: neutron_domain.clone(),
                addr: None,
            },
        );
    }

    // Amount we will LP for each token
    let lp_amount = 1_000_000u128;

    info!("Inserting all libraries...");
    // Reverse splitter will take tokenfactory token from account 1 and NTRN from account 2 and send it to account 3
    program_config.libraries.insert(
        1,
        LibraryInfo {
            name: "reverse_splitter".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceReverseSplitterLibrary(
                valence_reverse_splitter_library::msg::LibraryConfig {
                    output_addr: LibraryAccountType::AccountId(3),
                    splits: vec![
                        valence_reverse_splitter_library::msg::UncheckedSplitConfig {
                            denom: UncheckedDenom::Native(token.clone()),
                            account: LibraryAccountType::AccountId(1),
                            amount:
                                valence_reverse_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                                    lp_amount.into(),
                                ),
                            factor: None,
                        },
                        valence_reverse_splitter_library::msg::UncheckedSplitConfig {
                            denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                            account: LibraryAccountType::AccountId(2),
                            amount:
                                valence_reverse_splitter_library::msg::UncheckedSplitAmount::FixedAmount(
                                    lp_amount.into(),
                                ),
                            factor: None,
                        }
                    ],
                    base_denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                },
            ),
            addr: None,
        },
    );
    // LP forwarder will forward the joint deposit to an LP account
    program_config.libraries.insert(
        2,
        LibraryInfo {
            name: "lp_forwarder".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceForwarderLibrary(
                valence_forwarder_library::msg::LibraryConfig {
                    input_addr: LibraryAccountType::AccountId(3),
                    output_addr: LibraryAccountType::AccountId(4),
                    forwarding_configs: vec![
                        UncheckedForwardingConfig {
                            denom: UncheckedDenom::Native(token.clone()),
                            max_amount: Uint128::new(lp_amount),
                        },
                        UncheckedForwardingConfig {
                            denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                            max_amount: Uint128::new(lp_amount),
                        },
                    ],
                    forwarding_constraints: ForwardingConstraints::new(None),
                },
            ),
            addr: None,
        },
    );
    // The Astroport LPer will LP the tokens and deposit them in the LP deposit account
    program_config.libraries.insert(
        3,
        LibraryInfo {
            name: "astroport_lper".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceAstroportLper(
                valence_astroport_lper::msg::LibraryConfig {
                    input_addr: LibraryAccountType::AccountId(4),
                    output_addr: LibraryAccountType::AccountId(5),
                    pool_addr: pool_addr.to_string(),
                    lp_config: LiquidityProviderConfig {
                        pool_type: valence_astroport_lper::msg::PoolType::NativeLpToken(
                            PairType::Xyk {},
                        ),
                        asset_data: AssetData {
                            asset1: NTRN_DENOM.to_string(),
                            asset2: token.clone(),
                        },
                        max_spread: None,
                    },
                },
            ),
            addr: None,
        },
    );
    // The LP position forwarder will forward the LP position to the Available LP tokens account
    program_config.libraries.insert(
        4,
        LibraryInfo {
            name: "lp_position_forwarder".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceForwarderLibrary(
                valence_forwarder_library::msg::LibraryConfig {
                    input_addr: LibraryAccountType::AccountId(5),
                    output_addr: LibraryAccountType::AccountId(6),
                    forwarding_configs: vec![UncheckedForwardingConfig {
                        denom: UncheckedDenom::Native(lp_token.to_string()),
                        max_amount: Uint128::new(u128::MAX),
                    }],
                    forwarding_constraints: ForwardingConstraints::new(None),
                },
            ),
            addr: None,
        },
    );
    // The available LP tokens forwarder will forward the available LP tokens to the LP withdrawer account
    program_config.libraries.insert(
        5,
        LibraryInfo {
            name: "available_lp_tokens_forwarder".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceForwarderLibrary(
                valence_forwarder_library::msg::LibraryConfig {
                    input_addr: LibraryAccountType::AccountId(6),
                    output_addr: LibraryAccountType::AccountId(7),
                    forwarding_configs: vec![UncheckedForwardingConfig {
                        denom: UncheckedDenom::Native(lp_token.to_string()),
                        max_amount: Uint128::new(u128::MAX),
                    }],
                    forwarding_constraints: ForwardingConstraints::new(None),
                },
            ),
            addr: None,
        },
    );
    // The Astroport withdrawer will withdraw the liquidity and send it to the withdrawal account
    program_config.libraries.insert(
        6,
        LibraryInfo {
            name: "astroport_withdrawer".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceAstroportWithdrawer(
                valence_astroport_withdrawer::msg::LibraryConfig {
                    input_addr: LibraryAccountType::AccountId(7),
                    output_addr: LibraryAccountType::AccountId(8),
                    pool_addr: pool_addr.to_string(),
                    withdrawer_config:
                        valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
                            pool_type: valence_astroport_withdrawer::msg::PoolType::NativeLpToken,
                        },
                },
            ),
            addr: None,
        },
    );
    // The splitter will split the liquidity for the Tokenfactory Token and NTRN receiver accounts
    program_config.libraries.insert(
        7,
        LibraryInfo {
            name: "splitter".to_string(),
            domain: neutron_domain.clone(),
            config: LibraryConfig::ValenceSplitterLibrary(
                valence_splitter_library::msg::LibraryConfig {
                    input_addr: LibraryAccountType::AccountId(8),
                    splits: vec![
                        valence_splitter_library::msg::UncheckedSplitConfig {
                            denom: UncheckedDenom::Native(token.clone()),
                            account: LibraryAccountType::AccountId(9),
                            amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                                Decimal::percent(100),
                            ),
                        },
                        valence_splitter_library::msg::UncheckedSplitConfig {
                            denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                            account: LibraryAccountType::AccountId(10),
                            amount: valence_splitter_library::msg::UncheckedSplitAmount::FixedRatio(
                                Decimal::percent(100),
                            ),
                        },
                    ],
                },
            ),
            addr: None,
        },
    );

    info!("Inserting links...");
    // The depositors will deposit into the joint account
    program_config.links.insert(
        1,
        Link {
            input_accounts_id: vec![1, 2],
            output_accounts_id: vec![3],
            library_id: 1,
        },
    );
    // The LP forwarder will forward the joint deposit to the LP account
    program_config.links.insert(
        2,
        Link {
            input_accounts_id: vec![3],
            output_accounts_id: vec![4],
            library_id: 2,
        },
    );
    // The joint account will forward the tokens to the LP account
    program_config.links.insert(
        3,
        Link {
            input_accounts_id: vec![4],
            output_accounts_id: vec![5],
            library_id: 3,
        },
    );
    // The LP position account will forward the LP position to the available LP tokens account
    program_config.links.insert(
        4,
        Link {
            input_accounts_id: vec![5],
            output_accounts_id: vec![6],
            library_id: 4,
        },
    );
    // The available LP tokens account will forward the available LP tokens to the LP withdrawer account
    program_config.links.insert(
        5,
        Link {
            input_accounts_id: vec![6],
            output_accounts_id: vec![7],
            library_id: 5,
        },
    );
    // The LP withdrawer account will withdraw the liquidity and send it to the withdrawal account
    program_config.links.insert(
        6,
        Link {
            input_accounts_id: vec![7],
            output_accounts_id: vec![8],
            library_id: 6,
        },
    );
    // The splitter will split the liquidity for the Tokenfactory Token and NTRN receiver accounts
    program_config.links.insert(
        7,
        Link {
            input_accounts_id: vec![8],
            output_accounts_id: vec![9, 10],
            library_id: 7,
        },
    );

    info!("Adding authorizations...");
    program_config.authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("split_deposit")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(1))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "split".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("provide_liquidity")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(2))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "forward".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(3))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "provide_double_sided_liquidity".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(3))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "provide_single_sided_liquidity".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("forward_lp_position")
            .with_mode(AuthorizationModeInfo::Permissioned(
                PermissionTypeInfo::WithoutCallLimit(vec![NEUTRON_USER_ADDRESS_1.to_string()]),
            ))
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(4))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "forward".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("withdraw_liquidity")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(5))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "forward".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(6))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "withdraw_liquidity".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::LibraryId(7))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "split".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
    ];

    info!("Creating the program...");
    program_config.verify_new_config()?;
    setup_manager(
        &mut test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![
            SPLITTER_NAME,
            REVERSE_SPLITTER_NAME,
            FORWARDER_NAME,
            ASTROPORT_LPER_NAME,
            ASTROPORT_WITHDRAWER_NAME,
        ],
    )?;
    use_manager_init(&mut program_config)?;

    // Get addresses that we need to start
    let authorization_contract_address = program_config.authorization_data.authorization_addr;
    let processor_contract_address = program_config
        .authorization_data
        .processor_addrs
        .get(&neutron_domain.to_string())
        .unwrap()
        .clone();
    let tokenfactory_depositor = program_config
        .accounts
        .get(&1)
        .unwrap()
        .addr
        .clone()
        .unwrap();
    let neutron_depositor = program_config
        .accounts
        .get(&2)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    info!("Fund the depositor accounts with the required tokens...");

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &tokenfactory_depositor,
        &[Coin {
            denom: token.clone(),
            amount: lp_amount.into(),
        }],
        &Coin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &neutron_depositor,
        &[Coin {
            denom: NTRN_DENOM.to_string(),
            amount: lp_amount.into(),
        }],
        &Coin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    info!("Depositor accounts funded successfully!");

    info!("Sending message to reverse split to authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_reverse_splitter_library::msg::FunctionMsgs::Split {},
            ),
        )
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "split_deposit".to_string(),
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
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Ticking processor...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Verifying joint deposit balance...");
    let joint_deposit_address = program_config
        .accounts
        .get(&3)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &joint_deposit_address,
    );

    assert!(balance
        .iter()
        .any(|balance| balance.denom == token && balance.amount.u128() == lp_amount));
    assert!(balance
        .iter()
        .any(|balance| balance.denom == *NTRN_DENOM && balance.amount.u128() == lp_amount));

    info!("Sending messages to provide liquidity...");
    let binary1 = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_forwarder_library::msg::FunctionMsgs::Forward {},
            ),
        )
        .unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary1 };

    let binary2 = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideDoubleSidedLiquidity {
                    expected_pool_ratio_range: None,
                },
            ),
        )
        .unwrap(),
    );
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary2 };

    let binary3 = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_lper::msg::FunctionMsgs::ProvideSingleSidedLiquidity {
                    asset: NTRN_DENOM.to_string(),
                    limit: None,
                    expected_pool_ratio_range: None,
                },
            ),
        )
        .unwrap(),
    );
    let message3 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary3 };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "provide_liquidity".to_string(),
            messages: vec![message1, message2, message3],
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
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Ticking processor...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Verifying LP position account...");
    let lp_position_account_address = program_config
        .accounts
        .get(&5)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &lp_position_account_address,
    );

    assert!(balance
        .iter()
        .any(|balance| balance.denom.ends_with(ASTROPORT_LP_SUBDENOM)));

    info!("Sending message to forward LP position...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_forwarder_library::msg::FunctionMsgs::Forward {},
            ),
        )
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "forward_lp_position".to_string(),
            messages: vec![message],
            ttl: None,
        },
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &authorization_contract_address,
        USER_KEY_1,
        &serde_json::to_string(&send_msg).unwrap(),
        GAS_FLAGS,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Ticking processor...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    info!("Verifying available LP tokens account...");
    let available_lp_tokens_account_address = program_config
        .accounts
        .get(&6)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    let balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &available_lp_tokens_account_address,
    );

    assert!(balance
        .iter()
        .any(|balance| balance.denom.ends_with(ASTROPORT_LP_SUBDENOM)));

    info!("Sending message to withdraw liquidity...");
    let binary1 = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_forwarder_library::msg::FunctionMsgs::Forward {},
            ),
        )
        .unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary1 };

    let binary2 = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_astroport_withdrawer::msg::FunctionMsgs::WithdrawLiquidity {},
            ),
        )
        .unwrap(),
    );
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary2 };

    let binary3 = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_splitter_library::msg::FunctionMsgs::Split {},
            ),
        )
        .unwrap(),
    );
    let message3 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary3 };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "withdraw_liquidity".to_string(),
            messages: vec![message1, message2, message3],
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
    std::thread::sleep(std::time::Duration::from_secs(2));

    info!("Ticking processor...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    info!("Verifying final balances...");
    let tokenfactory_token_receiver = program_config
        .accounts
        .get(&9)
        .unwrap()
        .addr
        .clone()
        .unwrap();
    let neutron_receiver = program_config
        .accounts
        .get(&10)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    let tokenfactory_token_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &tokenfactory_token_receiver,
    );

    let neutron_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &neutron_receiver,
    );

    assert!(tokenfactory_token_balance.len() == 1);
    assert!(neutron_balance.len() == 1);
    assert!(tokenfactory_token_balance[0].denom == token);
    assert!(neutron_balance[0].denom == *NTRN_DENOM);

    info!("Finished Two Party Single Domain (Neutron) Astroport POL tests!");

    Ok(())
}
