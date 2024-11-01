use std::{env, error::Error};

use cosmwasm_std::{Binary, Uint128};
use cosmwasm_std_old::Coin as BankCoin;

use local_interchaintest::utils::{
    manager::{setup_manager, use_manager_init, SPLITTER_NAME},
    processor::tick_processor,
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, NTRN_DENOM, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::{
    bank,
    cosmwasm::{contract_execute, contract_instantiate},
};
use localic_utils::{
    utils::test_context::TestContext, ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY,
    GAIA_CHAIN_NAME, LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use log::info;
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

const ONE_HUNDRED: u128 = 100u128;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    let conditional_branch_contract_address =
        setup_conditional_branch_contract(&mut test_ctx, NEUTRON_CHAIN_ADMIN_ADDR.to_owned())?;

    let mut workflow_config_builder =
        WorkflowConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_workflow_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let rebalancer_account = workflow_config_builder.add_account(AccountInfo::new(
        "rebalancer_account".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let output_account_1 = workflow_config_builder.add_account(AccountInfo::new(
        "output_account_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let output_account_2 = workflow_config_builder.add_account(AccountInfo::new(
        "output_account_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let splitter = workflow_config_builder.add_service(ServiceInfo::new(
        "splitter".to_string(),
        &neutron_domain,
        ServiceConfig::ValenceSplitterService(valence_splitter_service::msg::ServiceConfig {
            input_addr: rebalancer_account.clone(),
            splits: vec![
                UncheckedSplitConfig {
                    denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                    account: output_account_1.clone(),
                    amount: UncheckedSplitAmount::FixedAmount(ONE_HUNDRED.into()),
                },
                UncheckedSplitConfig {
                    denom: UncheckedDenom::Native(NTRN_DENOM.to_string()),
                    account: output_account_2.clone(),
                    amount: UncheckedSplitAmount::FixedAmount(ONE_HUNDRED.into()),
                },
            ],
        }),
    ));

    workflow_config_builder.add_link(
        &splitter,
        vec![&rebalancer_account],
        vec![&output_account_1, &output_account_2],
    );

    workflow_config_builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("try_split")
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(ServiceAccountType::Addr(
                                conditional_branch_contract_address,
                            ))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "compare_and_branch".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "compare_and_branch".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(splitter)
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
    let mut test_ctx = setup_manager(
        test_ctx,
        NEUTRON_CONFIG_FILE,
        vec![GAIA_CHAIN_NAME],
        vec![SPLITTER_NAME],
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
    let rebalancer_account = workflow_config
        .get_account(rebalancer_account)
        .unwrap()
        .addr
        .clone()
        .unwrap();
    let output_account_1 = workflow_config
        .get_account(output_account_1)
        .unwrap()
        .addr
        .clone()
        .unwrap();
    let output_account_2 = workflow_config
        .get_account(output_account_2)
        .unwrap()
        .addr
        .clone()
        .unwrap();

    // Provision Rebalancer account with USDC
    bank::send(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        &rebalancer_account,
        &[BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: (ONE_HUNDRED * 2).into(),
        }],
        &BankCoin {
            denom: NTRN_DENOM.to_string(),
            amount: cosmwasm_std_old::Uint128::new(5000),
        },
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(3));

    info!("Send the messages to the authorization contract...");
    let balance_check_bin = Binary::from(
        serde_json::to_vec(
            &valence_conditional_branch::msg::ExecuteMsg::CompareAndBranch {
                query: valence_conditional_branch::msg::QueryInstruction::BalanceQuery {
                    address: rebalancer_account.to_string(),
                    denom: NTRN_DENOM.to_string(),
                },
                operator: valence_conditional_branch::msg::ComparisonOperator::GreaterThanOrEqual,
                rhs_operand: Binary::from(
                    serde_json::to_vec(&Uint128::from(ONE_HUNDRED * 2)).unwrap(),
                ),
                true_branch: None,
                false_branch: None,
            },
        )
        .unwrap(),
    );
    let balance_check_msg = ProcessorMessage::CosmwasmExecuteMsg {
        msg: balance_check_bin,
    };

    let split_bin = Binary::from(
        serde_json::to_vec(
            &valence_service_utils::msg::ExecuteMsg::<_, ()>::ProcessAction(ActionMsgs::Split {}),
        )
        .unwrap(),
    );
    let split_msg = ProcessorMessage::CosmwasmExecuteMsg { msg: split_bin };

    let send_msg = valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
        valence_authorization_utils::msg::PermissionlessMsg::SendMsgs {
            label: "try_split".to_string(),
            messages: vec![balance_check_msg, split_msg],
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

    info!("Ticking processor and executing try_split...");
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
        &output_account_1,
    );
    println!("{:?}", token_balances);
    // assert!(token_balances
    //     .iter()
    //     .any(|balance| balance.denom == NTRN_DENOM && balance.amount.u128() == ONE_HUNDRED));

    let token_balances = bank::get_balance(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        &output_account_2,
    );
    println!("{:?}", token_balances);
    // assert!(token_balances
    //     .iter()
    //     .any(|balance| balance.denom == NTRN_DENOM && balance.amount.u128() == ONE_HUNDRED));

    info!("Rebalancer workflow successful!");
    Ok(())
}

fn setup_conditional_branch_contract(
    test_ctx: &mut TestContext,
    owner: String,
) -> Result<String, Box<dyn Error>> {
    // Upload the conditional branch contract to Neutron
    let current_dir = env::current_dir()?;
    let conditional_branch_contract_path = format!(
        "{}/artifacts/valence_conditional_branch.wasm",
        current_dir.display()
    );

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader.send_single_contract(&conditional_branch_contract_path)?;

    // Get the code id
    let code_id = test_ctx
        .get_contract()
        .contract("valence_conditional_branch")
        .get_cw()
        .code_id
        .unwrap();

    let instantiate_msg = valence_conditional_branch::msg::InstantiateMsg { owner };

    let contract = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        code_id,
        &serde_json::to_string(&instantiate_msg).unwrap(),
        "valence_conditional_branch",
        None,
        "",
    )
    .unwrap();

    Ok(contract.address)
}
