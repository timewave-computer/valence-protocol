#[cfg(test)]
mod test {
    use cosmwasm_std::Uint128;
    use cw_ownable::Expiration;
    use std::collections::BTreeMap;

    use serde_json_any_key::MapIterToJson;
    use valence_authorization_utils::{
        action::AtomicAction,
        authorization::{
            ActionsConfig, AtomicActionsConfig, AuthorizationDuration, AuthorizationInfo,
            AuthorizationModeInfo,
        },
        authorization_message::{Message, MessageDetails, MessageType},
    };
    use valence_service_utils::{denoms::UncheckedDenom, ServiceAccountType};

    use crate::{
        account::{AccountInfo, AccountType},
        config::Config,
        domain::Domain,
        service::{ServiceConfig, ServiceInfo},
        workflow_config::{Link, WorkflowConfig},
    };

    /// test to make sure on config is parsed correctlly.
    /// MUST fix this test before handling other tests, config is part of the context we use, if we can't generate it successfully
    /// probably means other tests are also failing because of it.
    #[tokio::test]
    async fn test_config() {
        let _config = Config::default();
    }

    #[ignore = "internal test"]
    #[test]
    fn test_domain_ser() {
        // Make sure to_string returns the correct string
        let domain_string = Domain::CosmosCosmwasm("neutron".to_string()).to_string();
        assert_eq!(domain_string, "CosmosCosmwasm:neutron");

        // Make sure from_string returns the correct domain
        let domain = Domain::from_string(domain_string.clone()).unwrap();
        assert_eq!(domain, Domain::CosmosCosmwasm("neutron".to_string()));
    }

    #[ignore = "internal test"]
    #[tokio::test]
    async fn test_domains() {
        // let domain = Domain::CosmosCosmwasm("neutron");
        // let mut connector = domain.generate_connector().await.unwrap();
        // let (addr, salt) = connector
        //     .get_address(2, "splitter", "splitter")
        //     .await
        //     .unwrap();
    }

    #[ignore = "internal test"]
    #[test]
    fn test_config_find_accounts_ids() {
        let config = ServiceConfig::Forwarder(valence_forwarder_service::msg::ServiceConfig {
            input_addr: "|account_id|:1".into(),
            output_addr: "|account_id|:2".into(),
            forwarding_configs: vec![valence_forwarder_service::msg::UncheckedForwardingConfig {
                denom: UncheckedDenom::Native("untrn".to_string()),
                max_amount: Uint128::new(100),
            }],
            forwarding_constraints: valence_forwarder_service::msg::ForwardingConstraints::new(
                None,
            ),
        });

        let account_ids = config.get_account_ids().unwrap();
        println!("{account_ids:?}");
    }

    #[ignore = "internal test"]
    #[test]
    fn test_serialize_workflow() {}

    #[test]
    fn test_serialize() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
        struct Test {
            test: ServiceAccountType,
        }

        let t = Test {
            test: ServiceAccountType::AccountId(1),
        };

        // let mut json = serde_json::to_string(&t).unwrap();
        let mut json = serde_json::to_string(&t).unwrap();
        println!("{json:?}");

        json = json.replace("|account_id|\":1", "service_account_addr\":\"cosmos1\"");
        println!("{json:?}");

        let back_struct = serde_json::from_str::<Test>(&json).unwrap();
        println!("{back_struct:?}");

        let mut splits: BTreeMap<ServiceAccountType, Uint128> = BTreeMap::new();
        splits.insert(ServiceAccountType::AccountId(2), 100_u128.into());
        splits.insert(ServiceAccountType::AccountId(3), 200_u128.into());
        // let to_vec = splits;
        let json = splits.to_json_map().unwrap();
        // let json = serde_json::to_string(&splits).unwrap();
        println!("{json:?}");
    }

    #[ignore = "internal test"]
    #[tokio::test]
    async fn test_full_workflow() {
        // let subscriber = tracing_subscriber::fmt()
        //     .with_max_level(tracing::Level::DEBUG)
        //     .with_test_writer()
        //     .with_span_events(FmtSpan::CLOSE)
        //     .finish();
        // tracing::subscriber::set_global_default(subscriber)
        //     .expect("setting default subscriber failed");
        let neutron_domain = Domain::CosmosCosmwasm("neutron".to_string());

        let mut config = WorkflowConfig {
            owner: "neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe".to_string(),
            ..Default::default()
        };

        config.accounts.insert(
            1,
            AccountInfo {
                name: "test_1".to_string(),
                ty: AccountType::Base { admin: None },
                domain: neutron_domain.clone(),
            },
        );
        config.accounts.insert(
            2,
            AccountInfo {
                name: "test_2".to_string(),
                ty: AccountType::Base { admin: None },
                domain: neutron_domain.clone(),
            },
        );

        config.services.insert(
            1,
            ServiceInfo {
                name: "test_forwarder".to_string(),
                domain: neutron_domain.clone(),
                config: ServiceConfig::Forwarder(valence_forwarder_service::msg::ServiceConfig {
                    input_addr: ServiceAccountType::AccountId(1),
                    output_addr: ServiceAccountType::AccountId(2),
                    forwarding_configs: vec![
                        valence_forwarder_service::msg::UncheckedForwardingConfig {
                            denom: UncheckedDenom::Native("untrn".to_string()),
                            max_amount: Uint128::new(100),
                        },
                    ],
                    forwarding_constraints:
                        valence_forwarder_service::msg::ForwardingConstraints::new(None),
                }),
                addr: None,
            },
        );

        config.links.insert(
            1,
            Link {
                input_accounts_id: vec![1],
                output_accounts_id: vec![2],
                service_id: 1,
            },
        );

        // TODO: we need the id of the service here in contract_address
        config.authorizations.insert(
            1,
            AuthorizationInfo {
                label: "test".to_string(),
                mode: AuthorizationModeInfo::Permissionless,
                not_before: Expiration::Never {},
                duration: AuthorizationDuration::Forever,
                max_concurrent_executions: None,
                actions_config: ActionsConfig::Atomic(AtomicActionsConfig {
                    actions: vec![AtomicAction {
                        domain: valence_authorization_utils::domain::Domain::Main,
                        message_details: MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "test".to_string(),
                                params_restrictions: None,
                            },
                        },
                        contract_address: ServiceAccountType::ServiceId(1),
                    }],
                    retry_logic: None,
                }),
                priority: None,
            },
        );

        // config.authorization_data.set_processor_bridge_addr(Domain::CosmosCosmwasm("neutron".to_string()), "sdf".to_string());

        // let b = to_json_binary(&config).unwrap();
        // println!("{:#?}", b);

        // init_workflow(config).await;

        // match timeout(Duration::from_secs(60), ).await {
        //     Ok(_) => println!("Workflow initialization completed successfully"),
        //     Err(_) => println!("Workflow initialization timed out after 60 seconds"),
        // }
    }
}
