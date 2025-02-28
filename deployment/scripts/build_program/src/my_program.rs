use std::{error::Error};
use rand::{distributions::Alphanumeric, Rng};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME, DEFAULT_KEY
};
use cosmwasm_std::{Binary, Decimal, Uint128};
use localic_std::modules::{
    cosmwasm::{contract_execute, contract_instantiate, contract_query},
};
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config_builder::ProgramConfigBuilder,
    program_config::ProgramConfig,
};
use valence_authorization_utils::{
    authorization::{
      AuthorizationModeInfo,
        PermissionTypeInfo
    },
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_splitter_library::msg::UncheckedSplitAmount;
use valence_splitter_library::msg::UncheckedSplitConfig;
use valence_library_utils::denoms::UncheckedDenom;
use valence_e2e::utils::{
    manager::{setup_manager, use_manager_init, SPLITTER_NAME},
    processor::tick_processor,
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};


// three accounts. let's call them input account, position account, output account
// two libraries. astroport liquidity provider and astroport withdrawer
// two subroutines:
// provide liquidity from the input account and deposit LP tokens into the position account
// withdraw liquidity from the position account and into the output account

/// Write your program using the program builder
pub(crate) fn my_program() -> Result<ProgramConfig, Box<dyn Error>> {

    let mut test_ctx = TestContextBuilder::default()
    .with_unwrap_raw_logs(true)
    .with_api_url(LOCAL_IC_API_URL)
    .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
    .with_chain(ConfigChainBuilder::default_neutron().build()?)
    .with_log_file_path(LOGS_FILE_PATH)
    .build()?;

    let token1_subdenom: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();


    let token2_subdenom: String = rand::thread_rng()
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
        .subdenom(token1_subdenom)
        .get();

    test_ctx
        .build_tx_create_tokenfactory_token()
        .with_subdenom(&token2_subdenom)
        .send()?;
    std::thread::sleep(std::time::Duration::from_secs(3));

    let token2 = test_ctx
        .get_tokenfactory_denom()
        .creator(NEUTRON_CHAIN_ADMIN_ADDR)
        .subdenom(token2_subdenom)
        .get();

    let swap_amount = 1_000_000_000;
    let swap_amount_uint128 = Uint128::new(swap_amount as u128);

    let permissioned_addr="";

    let mut program_config_builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let account_1 = program_config_builder.add_account(AccountInfo::new(
        "input_account_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_2 = program_config_builder.add_account(AccountInfo::new(
        "input_account_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

 
   
    let library_1 = program_config_builder.add_library(LibraryInfo::new(
        "splitter_1".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(valence_splitter_library::msg::LibraryConfig {
            input_addr: account_1.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token1.to_string()),
                account: account_2.clone(),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount_uint128.into()),
            }],
        }),
    ));

    let library_2 = program_config_builder.add_library(LibraryInfo::new(
        "splitter_2".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(valence_splitter_library::msg::LibraryConfig {
            input_addr: account_2.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token2.to_string()),
                account: account_1.clone(),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount_uint128.into()),
            }],
        }),
    ));

    program_config_builder.add_link(&library_1, vec![&account_1], vec![&account_2]);
    program_config_builder.add_link(&library_2, vec![&account_2], vec![&account_1]);




    program_config_builder.add_authorization(
        AuthorizationBuilder::new()
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![
                permissioned_addr.to_string(),
            ])
        )
    )
            .with_label("swap1")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(library_1.clone())
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
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(library_2.clone())
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
    );

    program_config_builder.add_authorization(
        AuthorizationBuilder::new()
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithCallLimit(vec![(
                permissioned_addr.to_string(),
                Uint128::new(5),
            )]),
        ))
        
            .with_label("swap2")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
            
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(library_1.clone())
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
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(library_2.clone())
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
    );



     Ok(program_config_builder.build())
    
}
