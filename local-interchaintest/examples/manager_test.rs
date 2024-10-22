use std::error::Error;

use local_interchaintest::utils::{
    manager::{setup_manager, use_manager_init, SPLITTER_NAME},
    LOGS_FILE_PATH, NEUTRON_CONFIG_FILE, VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, GAIA_CHAIN_NAME, LOCAL_IC_API_URL,
    NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME,
};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder},
};
use valence_service_utils::denoms::UncheckedDenom;
use valence_splitter_service::msg::{UncheckedSplitAmount, UncheckedSplitConfig};
use valence_workflow_manager::{
    account::{AccountInfo, AccountType},
    service::{ServiceConfig, ServiceInfo},
    workflow_config_builder::WorkflowConfigBuilder,
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
    let service_1 = builder.add_service(ServiceInfo {
        name: "test_splitter".to_string(),
        domain: neutron_domain.clone(),
        config: ServiceConfig::ValenceSplitterService(
            valence_splitter_service::msg::ServiceConfig {
                input_addr: account_1.clone(),
                splits: vec![UncheckedSplitConfig {
                    denom: UncheckedDenom::Native("test".to_string()),
                    account: account_2.clone(),
                    amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
                }],
            },
        ),
        addr: None,
    });

    builder.add_link(&service_1, vec![&account_1], vec![&account_2]);

    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label("swap")
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_contract_address(service_1)
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

    Ok(())
}
