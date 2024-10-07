#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use cosmwasm_std::Uint128;
    use cw_ownable::Expiration;
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
    #[tokio::test]
    async fn test_domains() {
        // let _profiler = dhat::Profiler::builder().testing().build();

        let _config = Config::default();
        // let ctx = Connectors::new(&config);

        let domain = Domain::CosmosCosmwasm("neutron".to_string());
        let mut _neutron_connector = domain.generate_connector().await.unwrap();

        // let workflow_id = neutron_connector.reserve_workflow_id().await.unwrap();
        let workflow_id = 4; // hardcoded for testing, we do not actually need to reserve a workflow id for testing because we don't save the id for now

        let auth_addr = "neutron1psdjqpycm9cpqzu8av9ycepdmarq82dzh3p2ckg6r3y74v456mvsd5racz";
        let processor_addr = "neutron106wur25r9asjeel9wumvlpw0h4fmmkt387gdjrgz5zgc2munv9js26ua7j";
        let account_addr = "neutron1hpkn6y0tn4gdlyxc5pl3qcfcqfrg5y6q92upyk3xdk4y69vd63pqjhgmwq";
        let forwarder_addr = "neutron1dzx7zsljlf38x8jyhstk5ts58x6yyd450x62aaejk9cn4cskyvlsd7lchu";

        // // init auth contract
        // let (auth_addr, auth_salt) = neutron_connector
        //     .get_address(workflow_id, "authorization", "authorization")
        //     .await
        //     .unwrap();
        // let (processor_addr, processor_salt) = neutron_connector
        //     .get_address(workflow_id, "processor", "processor")
        //     .await
        //     .unwrap();
        // let (account_addr, account_salt) = neutron_connector
        //     .get_address(workflow_id, "base_account", "account:1")
        //     .await
        //     .unwrap();

        // neutron_connector
        //     .instantiate_authorization(workflow_id, auth_salt, processor_addr.clone())
        //     .await
        //     .unwrap();
        // neutron_connector
        //     .instantiate_processor(workflow_id, processor_salt, auth_addr.clone(), None)
        //     .await
        //     .unwrap();
        // neutron_connector
        //     .instantiate_account(
        //         workflow_id,
        //         processor_addr.clone(),
        //         &InstantiateAccountData {
        //             id: 1,
        //             info: AccountInfo {
        //                 name: "Test account".to_string(),
        //                 ty: AccountType::Base { admin: None },
        //                 domain,
        //             },
        //             addr: account_addr.clone(),
        //             salt: account_salt,
        //             approved_services: vec![],
        //         },
        //     )
        //     .await
        //     .unwrap();

        // let (forwarder_addr, forwarder_salt) = neutron_connector
        //     .get_address(workflow_id, "forwarder", "forwarder:1")
        //     .await
        //     .unwrap();

        // neutron_connector
        //     .instantiate_service(
        //         workflow_id,
        //         auth_addr.to_string(),
        //         processor_addr.to_string(),
        //         1,
        //         ServiceConfig::Forwarder(valence_forwarder_service::msg::ServiceConfig {
        //             input_addr: account_addr.to_string(),
        //             output_addr: account_addr.to_string(),
        //             forwarding_configs: vec![
        //                 valence_forwarder_service::msg::UncheckedForwardingConfig {
        //                     denom: UncheckedDenom::Native("untrn".to_string()),
        //                     max_amount: Uint128::new(100),
        //                 },
        //             ],
        //             forwarding_constraints:
        //                 valence_forwarder_service::msg::ForwardingConstraints::new(None),
        //         }),
        //         forwarder_salt,
        //     )
        //     .await
        //     .unwrap();

        println!("id: {:?}", workflow_id);
        println!("auth_addr: {:?}", auth_addr);
        println!("processor_addr: {:?}", processor_addr);
        println!("account_addr: {:?}", account_addr);
        println!("service_addr: {:?}", forwarder_addr);
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

        json = json.replace("|account_id|\":1", "account_addr\":\"cosmos1\"");
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
                        contract_address:
                            "neutron1dzx7zsljlf38x8jyhstk5ts58x6yyd450x62aaejk9cn4cskyvlsd7lchu"
                                .to_string(),
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
