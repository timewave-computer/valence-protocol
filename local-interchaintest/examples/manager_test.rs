use std::{collections::BTreeMap, error::Error};

use cosmwasm_std::{CosmosMsg, WasmMsg};
use local_interchaintest::utils::{
    manager::{setup_manager, use_manager_init, use_manager_update, SPLITTER_NAME},
    GAS_FLAGS, LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use localic_std::modules::cosmwasm::{contract_execute, contract_query};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, DEFAULT_KEY, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use serde_json::Value;
use valence_authorization::contract::build_tokenfactory_denom;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder},
};
use valence_service_utils::{denoms::UncheckedDenom, GetId, Id, ServiceAccountType};
use valence_splitter_service::msg::{UncheckedSplitAmount, UncheckedSplitConfig};
use valence_workflow_manager::{
    account::{AccountInfo, AccountType},
    service::{ServiceConfig, ServiceConfigUpdate, ServiceInfo},
    workflow_config_builder::WorkflowConfigBuilder,
    workflow_update::{AuthorizationInfoUpdate, WorkflowConfigUpdate},
};

fn main() -> Result<(), Box<dyn Error>> {
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

    let mut builder = WorkflowConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_workflow_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

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

    let swap_amount: u128 = 1_000_000_000;
    let mut service_config = valence_splitter_service::msg::ServiceConfig {
        input_addr: account_1.clone(),
        splits: vec![UncheckedSplitConfig {
            denom: UncheckedDenom::Native("test".to_string()),
            account: account_2.clone(),
            amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
        }],
    };

    let service_1 = builder.add_service(ServiceInfo::new(
        "test_splitter".to_string(),
        &neutron_domain,
        ServiceConfig::ValenceSplitterService(service_config.clone()),
    ));

    builder.add_link(&service_1, vec![&account_1], vec![&account_2]);

    let action_label = "swap";
    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label(action_label)
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(service_1.clone())
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

    let mut workflow_config = builder.build();

    use_manager_init(&mut workflow_config)?;

    // Do the updates

    let splitter_data = workflow_config.get_service(service_1.get_id()).unwrap();
    let neutron_processor_addr = workflow_config
        .authorization_data
        .processor_addrs
        .get(&neutron_domain.to_string())
        .unwrap();
    let authorization_addr = workflow_config
        .authorization_data
        .authorization_addr
        .clone();

    // modify the service config to change the denom of the split
    service_config.splits[0].denom = UncheckedDenom::Native("test2".to_string());
    service_config.splits[0].account = ServiceAccountType::Addr(
        workflow_config
            .get_account(account_2.get_id())
            .unwrap()
            .clone()
            .addr
            .unwrap(),
    );

    let mut services_changes: BTreeMap<Id, ServiceConfigUpdate> = BTreeMap::new();
    services_changes.insert(
        service_1.get_id(),
        ServiceConfigUpdate::ValenceSplitterService(
            valence_splitter_service::msg::ServiceConfigUpdate {
                input_addr: None,
                splits: Some(service_config.splits),
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
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(service_1.clone())
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
    ));

    let update_config = WorkflowConfigUpdate {
        id: workflow_config.id,
        owner: None,
        services: services_changes,
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
        format!(
            "update_service_{}_{}",
            splitter_data.name,
            service_1.get_id()
        )
        .as_str(),
    );
    println!("Ticking processor with denom: {}", tick_denom);
    println!("auth addr {}", authorization_addr);

    // return Ok(());
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

    // assert service config changed
    let query_splitter_config_response: Value = serde_json::from_value(
        contract_query(
            test_ctx
                .get_request_builder()
                .get_request_builder(NEUTRON_CHAIN_NAME),
            &splitter_data.addr.unwrap(),
            &serde_json::to_string(
                &valence_splitter_service::msg::QueryMsg::GetRawServiceConfig {},
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

    Ok(())
}
