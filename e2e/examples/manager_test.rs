use std::{collections::BTreeMap, error::Error};

use cosmwasm_std::{Coin, CosmosMsg, WasmMsg};
use cosmwasm_std_old::Coin as BankCoin;
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_query},
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::Value;
use valence_authorization::contract::build_tokenfactory_denom;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_e2e::utils::{
    manager::{
        setup_manager, use_manager_init, use_manager_migrate, use_manager_update, SPLITTER_NAME,
    },
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use valence_library_utils::{
    denoms::UncheckedDenom, library_account_type::GetId, Id, LibraryAccountType,
};
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryConfigUpdate, LibraryInfo},
    program_config_builder::ProgramConfigBuilder,
    program_migration::{FundsTransfer, ProgramConfigMigrate},
    program_update::{AuthorizationInfoUpdate, ProgramConfigUpdate},
};
use valence_splitter_library::msg::{UncheckedSplitAmount, UncheckedSplitConfig};

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
        vec![SPLITTER_NAME],
    )?;

    // build the token1 denom
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
        .subdenom(token1_subdenom)
        .get();
    let swap_amount: u128 = 1_000_000_000;

    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(1_000_000_000_u128)
        .with_denom(&token1)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

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

    let mut library_config = valence_splitter_library::msg::LibraryConfig {
        input_addr: account_1.clone(),
        splits: vec![UncheckedSplitConfig {
            denom: UncheckedDenom::Native(token1.clone()),
            account: account_2.clone(),
            amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
        }],
    };

    let library_1 = builder.add_library(LibraryInfo::new(
        "test_splitter".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(library_config.clone()),
    ));

    builder.add_link(&library_1, vec![&account_1], vec![&account_2]);

    let action_label = "swap";
    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label(action_label)
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
                    .build(),
            )
            .build(),
    );

    let mut program_config = builder.build();

    use_manager_init(&mut program_config)?;

    let account_1_addr = program_config
        .get_account(account_1.get_account_id())
        .unwrap()
        .clone()
        .addr
        .unwrap();

    // We fund the account_1 with some test tokens for later tests
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &account_1_addr,
        &[BankCoin {
            denom: token1.clone(),
            amount: swap_amount.into(),
        }],
        &BankCoin {
            denom: NEUTRON_CHAIN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Do the updates

    let splitter_data = program_config
        .get_library(library_1.get_library_id())
        .unwrap();
    let neutron_processor_addr = program_config
        .authorization_data
        .processor_addrs
        .get(&neutron_domain.to_string())
        .unwrap();
    let authorization_addr = program_config.authorization_data.authorization_addr.clone();

    // modify the service config to change the denom of the split
    library_config.splits[0].denom = UncheckedDenom::Native("test2".to_string());
    library_config.splits[0].account = LibraryAccountType::Addr(account_1_addr.clone());

    let mut libraries_changes: BTreeMap<Id, LibraryConfigUpdate> = BTreeMap::new();
    libraries_changes.insert(
        library_1.get_library_id(),
        LibraryConfigUpdate::ValenceSplitterLibrary(
            valence_splitter_library::msg::LibraryConfigUpdate {
                input_addr: None,
                splits: Some(library_config.splits),
            },
        ),
    );

    // change authorizations
    let mut authorizations_changes = vec![AuthorizationInfoUpdate::Modify {
        label: action_label.to_string(),
        not_before: None,
        expiration: None,
        max_concurrent_executions: Some(10),
        priority: None,
    }];

    // add new authorization
    authorizations_changes.push(AuthorizationInfoUpdate::Add(
        AuthorizationBuilder::new()
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
                    .build(),
            )
            .build(),
    ));

    let update_config = ProgramConfigUpdate {
        id: program_config.id,
        owner: None,
        libraries: libraries_changes,
        authorizations: authorizations_changes,
    };

    let res = use_manager_update(update_config).unwrap();

    // apply updates
    for instruction in res.instructions.iter() {
        let (contract_addr, msg) = match instruction {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) => (contract_addr, core::str::from_utf8(msg.as_slice()).unwrap()),
            _ => panic!("Unexpected instruction type"),
        };

        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            contract_addr,
            DEFAULT_KEY,
            msg,
            GAS_FLAGS,
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // tick processor
    let tick_denom = build_tokenfactory_denom(
        &authorization_addr,
        format!("update_library_{}", library_1.get_library_id()).as_str(),
    );
    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        neutron_processor_addr,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        format!("{GAS_FLAGS} --amount 1{tick_denom}").as_str(),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // assert library config changed
    let query_splitter_config_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &splitter_data.addr.unwrap(),
            &serde_json::to_string(
                &valence_splitter_library::msg::QueryMsg::GetRawLibraryConfig {},
            )
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let split_denom = query_splitter_config_response["splits"][0]["denom"]
        .as_object()
        .unwrap()
        .get("native")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(split_denom == "test2");

    // asserts authorizations changed and added
    let query_authorizations_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &authorization_addr,
            &serde_json::to_string(
                &valence_authorization_utils::msg::QueryMsg::Authorizations {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let authorizations = query_authorizations_response.as_array().unwrap();

    assert!(authorizations.len() == 3);
    // TODO: Add more assertions that update was successful

    // For migration, we just want to just the migrate functionlity, so here are the steps we gonna take:
    // We have some "stuck" funds in our input account of the old program
    // Create the same program as above, same services and accounts
    // Migrate from old to new, with sending funds from the old input account to the new one

    // Create the new program
    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let new_account_1 = builder.add_account(AccountInfo::new(
        "account_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let new_account_2 = builder.add_account(AccountInfo::new(
        "account_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let library_config = valence_splitter_library::msg::LibraryConfig {
        input_addr: new_account_1.clone(),
        splits: vec![UncheckedSplitConfig {
            denom: UncheckedDenom::Native(token1.clone()),
            account: new_account_2.clone(),
            amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
        }],
    };

    let library_1 = builder.add_library(LibraryInfo::new(
        "test_splitter".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(library_config.clone()),
    ));

    builder.add_link(&library_1, vec![&new_account_1], vec![&new_account_2]);

    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label(action_label)
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
                    .build(),
            )
            .build(),
    );

    let new_program_config = builder.build();

    let transfers = vec![FundsTransfer {
        from: account_1_addr.clone(),
        to: new_account_1.clone(),
        domain: neutron_domain,
        funds: Coin {
            denom: token1.clone(),
            amount: swap_amount.into(),
        },
    }];

    let res = use_manager_migrate(ProgramConfigMigrate {
        old_id: program_config.id,
        new_program: new_program_config,
        transfer_funds: transfers,
    })
    .unwrap();

    for instruction in res.instructions.iter() {
        let (contract_addr, msg) = match instruction {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) => (contract_addr, core::str::from_utf8(msg.as_slice()).unwrap()),
            _ => panic!("Unexpected instruction type"),
        };

        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            contract_addr,
            DEFAULT_KEY,
            msg,
            GAS_FLAGS,
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // tick processor
    let tick_denom = build_tokenfactory_denom(
        &authorization_addr,
        format!("account_id_{}", account_1.get_account_id()).as_str(),
    );

    contract_execute(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        neutron_processor_addr,
        DEFAULT_KEY,
        &serde_json::to_string(
            &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                valence_processor_utils::msg::PermissionlessMsg::Tick {},
            ),
        )
        .unwrap(),
        format!("{GAS_FLAGS} --amount 1{tick_denom}").as_str(),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    // assert funds were transferred
    let new_account_1_addr = res
        .new_config
        .get_account(new_account_1.get_account_id())
        .unwrap()
        .clone()
        .addr
        .unwrap();

    let new_account_1_balance = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &new_account_1_addr,
    );

    assert!(new_account_1_balance
        .iter()
        .any(|b| b.denom == token1.clone() && b.amount.u128() == swap_amount));

    // Do the pause processors messages, after we transfered all of the funds
    for pause_processor in res.pause_processor_messages.iter() {
        let (contract_addr, msg) = match pause_processor {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr, msg, ..
            }) => (contract_addr, core::str::from_utf8(msg.as_slice()).unwrap()),
            _ => panic!("Unexpected pause_processor type"),
        };

        contract_execute(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            contract_addr,
            DEFAULT_KEY,
            msg,
            GAS_FLAGS,
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // asserts processor is paused
    let query_processor_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            neutron_processor_addr,
            &serde_json::to_string(&valence_processor_utils::msg::QueryMsg::Config {}).unwrap(),
        )["data"]
            .clone(),
    )
    .unwrap();

    let processor_status = query_processor_response["state"].as_str().unwrap();

    // assert!(processor_status == valence_processor_utils::processor::State::Paused.to_string());
    assert!(processor_status == "paused");

    Ok(())
}
