use std::error::Error;

use local_interchaintest::utils::{
    manager::{setup_manager, use_manager_init},
    LOGS_FILE_PATH, VALENCE_ARTIFACTS_PATH,
};
use localic_utils::{
    ConfigChainBuilder, TestContextBuilder, LOCAL_IC_API_URL, NEUTRON_CHAIN_ADMIN_ADDR,
    NEUTRON_CHAIN_NAME,
};
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder},
};
use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};
use valence_splitter_service::msg::{UncheckedSplitAmount, UncheckedSplitConfig};
use valence_workflow_manager::{
    account::{AccountInfo, AccountType},
    service::{ServiceConfig, ServiceInfo},
    workflow_config::{Link, WorkflowConfig},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut test_ctx = TestContextBuilder::default()
        .with_unwrap_raw_logs(true)
        .with_api_url(LOCAL_IC_API_URL)
        .with_artifacts_dir(VALENCE_ARTIFACTS_PATH)
        .with_chain(ConfigChainBuilder::default_neutron().build()?)
        .with_log_file_path(LOGS_FILE_PATH)
        .build()?;

    setup_manager(&mut test_ctx)?;

    let mut workflow_config = WorkflowConfig {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        ..Default::default()
    };
    let neutron_domain =
        valence_workflow_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    workflow_config.accounts.insert(
        1,
        AccountInfo {
            active: true,
            name: "test_1".to_string(),
            ty: AccountType::Base { admin: None },
            domain: neutron_domain.clone(),
            addr: None,
        },
    );
    workflow_config.accounts.insert(
        2,
        AccountInfo {
            active: true,
            name: "test_2".to_string(),
            ty: AccountType::Base { admin: None },
            domain: neutron_domain.clone(),
            addr: None,
        },
    );

    let swap_amount: u128 = 1_000_000_000;
    workflow_config.services.insert(
        1,
        ServiceInfo {
            active: true,
            name: "test_splitter".to_string(),
            domain: neutron_domain.clone(),
            config: ServiceConfig::ValenceSplitterService(
                valence_splitter_service::msg::ServiceConfig {
                    input_addr: ServiceAccountType::AccountId(1),
                    splits: vec![UncheckedSplitConfig {
                        denom: UncheckedDenom::Native("test".to_string()),
                        account: ServiceAccountType::AccountId(2),
                        amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
                    }],
                },
            ),
            addr: None,
        },
    );

    workflow_config.links.insert(
        1,
        Link {
            input_accounts_id: vec![1],
            output_accounts_id: vec![2],
            service_id: 1,
        },
    );

    workflow_config.authorizations = vec![AuthorizationBuilder::new()
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
                                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(
                                    vec!["process_action".to_string(), "split".to_string()],
                                )]),
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    use_manager_init(&mut workflow_config)?;

    Ok(())
}
