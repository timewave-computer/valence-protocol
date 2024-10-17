use std::error::Error;

use cosmwasm_std::Binary;
use cosmwasm_std_old::Coin as BankCoin;

use local_interchaintest::utils::{
    manager::{setup_manager, use_manager_init, SPLITTER_NAME},
    processor::tick_processor,
    GAS_FLAGS, LOGS_FILE_PATH, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{bank, cosmwasm::contract_execute};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
use rand::{distributions::Alphanumeric, Rng};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder},
    msg::ProcessorMessage,
};
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};
use valence_splitter_service::msg::{ActionMsgs, UncheckedSplitAmount, UncheckedSplitConfig};
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

    info!("Create and mint tokens to perform the swap...");
    // We are going to create 2 tokenfactory tokens so that we can test the token swap, one will be given to first account and the second one will be given to the second account

    // We are going to use random subdenoms so that the test can be run multiple times
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

    let mut workflow_config_builder =
        WorkflowConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_workflow_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let account_1 = workflow_config_builder.add_account(AccountInfo::new(
        "base_account_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let account_2 = workflow_config_builder.add_account(AccountInfo::new(
        "base_account_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let service_1 = workflow_config_builder.add_service(ServiceInfo::new(
        "splitter_1".to_string(),
        &neutron_domain,
        ServiceConfig::ValenceSplitterService(valence_splitter_service::msg::ServiceConfig {
            input_addr: account_1.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token1.to_string()),
                account: account_2.clone(),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
            }],
        }),
    ));

    let service_2 = workflow_config_builder.add_service(ServiceInfo::new(
        "splitter_2".to_string(),
        &neutron_domain,
        ServiceConfig::ValenceSplitterService(valence_splitter_service::msg::ServiceConfig {
            input_addr: account_2.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token2.to_string()),
                account: account_1.clone(),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
            }],
        }),
    ));

    workflow_config_builder.add_link(&service_1, vec![&account_1], vec![&account_2]);
    workflow_config_builder.add_link(&service_2, vec![&account_2], vec![&account_1]);

    workflow_config_builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("swap")
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(ServiceAccountType::ServiceId(1))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_action".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_action".to_string(),
                                            "split".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(ServiceAccountType::ServiceId(2))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_action".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_action".to_string(),
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

    let mut workflow_config = workflow_config_builder.build();

    // Verify config is ok before we upload all contracts
    workflow_config.verify_new_config()?;

    // Setup the contracts and update the global config
    info!("Setup manager...");
    setup_manager(
        &mut test_ctx,
        "neutron_juno.json",
        vec![SPLITTER_NAME.to_string()],
    )?;

    // init the workflow
    info!("Start manager init...");
    use_manager_init(&mut workflow_config)?;

    // Get all the addresses we need to interact with
    let authorization_contract_address = workflow_config
        .authorization_data
        .authorization_addr
        .clone();
    let processor_contract_address = workflow_config
        .get_processor_addr(&neutron_domain.to_string())
        .unwrap();
    let base_account_1 = workflow_config
        .get_account(account_1)
        .unwrap()
        .addr
        .clone()
        .unwrap();
    let base_account_2 = workflow_config
        .get_account(account_2)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    // Mint some of each token and send it to the base accounts
    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(swap_amount)
        .with_denom(&token1)
        .send()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &base_account_1,
        &[BankCoin {
            denom: token1.clone(),
            amount: swap_amount.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    test_ctx
        .build_tx_mint_tokenfactory_token()
        .with_amount(swap_amount)
        .with_denom(&token2)
        .send()
        .unwrap();

    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &base_account_2,
        &[BankCoin {
            denom: token2.clone(),
            amount: swap_amount.into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Send the messages to the authorization contract...");
    let binary = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(ActionMsgs::Split {}),
        )
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "swap".to_string(),
            messages: vec![message.clone(), message],
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

    info!("Ticking processor and executing swap...");
    tick_processor(
        &mut test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &processor_contract_address,
    );

    info!("Verifying balances...");
    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &base_account_1,
    );
    assert!(token_balances
        .iter()
        .any(|balance| balance.denom == token2 && balance.amount.u128() == swap_amount));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &base_account_2,
    );

    assert!(token_balances
        .iter()
        .any(|balance| balance.denom == token1 && balance.amount.u128() == swap_amount));

    info!("Token swap successful!");
    Ok(())
}
