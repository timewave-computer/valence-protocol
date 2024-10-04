#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use cosmwasm_std::Uint128;
    use serde_json_any_key::MapIterToJson;
    use valence_service_utils::ServiceAccountType;
    use valence_splitter_service::msg::ServiceConfig as SplitterServiceConfig;

    use crate::{
        account::{AccountInfo, AccountType},
        config::Config,
        domain::Domain,
        init_workflow,
        service::{ServiceConfig, ServiceInfo},
        workflow_config::{Link, WorkflowConfig},
    };

    /// test to make sure on config is parsed correctlly.
    /// MUST fix this test before handling other tests, config is part of the context we use, if we can't generate it successfully
    /// probably means other tests are also failing because of it.
    #[tokio::test]
    async fn test_config() {
        let config = Config::default();
        println!("{:#?}", config.general);
    }

    #[ignore = "internal test"]
    #[tokio::test]
    async fn test_domains() {
        // let _profiler = dhat::Profiler::builder().testing().build();

        let _config = Config::default();
        // let ctx = Connectors::new(&config);

        let domain = Domain::CosmosCosmwasm("neutron");
        let mut _connector = domain.generate_connector().await.unwrap();
        // let (addr, salt) = connector
        //     .get_address(1, "base_account", "account")
        //     .await
        //     .unwrap();
        // let account_info = InstantiateAccountData {
        //     id: 1,
        //     info: AccountInfo{ name: "Test".to_string(), ty: AccountType::Base { admin: None }, domain },
        //     addr,
        //     salt,
        //     approved_services: vec![],
        // };

        // let id = connector.reserve_workflow_id().await.unwrap();
        // println!("{:?}", id);

        // // let domain2 = Domain::Cosmos("neutron".to_string());
        // let mut domain_info = DomainInfo::from_domain(&domain).await;
        // println!("{domain_info:?}");
        // let mut domain_info2 = DomainInfo::from_domain(domain2).await;
        // println!("{domain_info2:?}");

        // let mut domain_info3 = DomainInfo::from_domain(domain).await;
        // println!("{domain_info3:?}");

        // let stats = dhat::HeapStats::get();

        // let d = domain_info
        //     .connector
        //     .init_account(
        //         1,
        //         None,
        //         "label".to_string(),
        //     )
        //     .await;
        // println!("Balance: {d:?}");
    }

    #[ignore = "internal test"]
    #[test]
    fn test_config_find_accounts_ids() {
        let config = ServiceConfig::Splitter(SplitterServiceConfig {
            input_addr: "|account_id|:1".to_string(),
            splits: vec![
                valence_splitter_service::msg::UncheckedSplitConfig::new(
                    valence_service_utils::denoms::UncheckedDenom::Native("NTRN".to_string()),
                    "|account_id|:2".to_string(),
                    Some(Uint128::from(1_000_000u128)),
                    None,
                ),
                valence_splitter_service::msg::UncheckedSplitConfig::new(
                    valence_service_utils::denoms::UncheckedDenom::Native("NTRN".to_string()),
                    "|account_id|:3".to_string(),
                    Some(Uint128::from(1_000_000u128)),
                    None,
                ),
            ],
        });

        let account_ids = config.get_account_ids().unwrap();
        println!("{account_ids:?}");
    }

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
    async fn test() {
        // let subscriber = tracing_subscriber::fmt()
        //     .with_max_level(tracing::Level::DEBUG)
        //     .with_test_writer()
        //     .with_span_events(FmtSpan::CLOSE)
        //     .finish();
        // tracing::subscriber::set_global_default(subscriber)
        //     .expect("setting default subscriber failed");

        let mut config = WorkflowConfig::default();

        config.accounts.insert(
            1,
            AccountInfo {
                name: "test_1".to_string(),
                ty: AccountType::Base { admin: None },
                domain: Domain::CosmosCosmwasm("neutron"),
            },
        );
        config.accounts.insert(
            2,
            AccountInfo {
                name: "test_2".to_string(),
                ty: AccountType::Base { admin: None },
                domain: Domain::CosmosCosmwasm("neutron"),
            },
        );
        config.accounts.insert(
            3,
            AccountInfo {
                name: "test_3".to_string(),
                ty: AccountType::Base { admin: None },
                domain: Domain::CosmosCosmwasm("neutron"),
            },
        );

        config.services.insert(
            1,
            ServiceInfo {
                name: "test_services".to_string(),
                domain: Domain::CosmosCosmwasm("neutron"),
                config: ServiceConfig::Splitter(SplitterServiceConfig {
                    input_addr: "|account_id|:1".to_string(),
                    splits: vec![
                        valence_splitter_service::msg::UncheckedSplitConfig::new(
                            valence_service_utils::denoms::UncheckedDenom::Native(
                                "NTRN".to_string(),
                            ),
                            "|account_id|:2".to_string(),
                            Some(Uint128::from(1_000_000u128)),
                            None,
                        ),
                        valence_splitter_service::msg::UncheckedSplitConfig::new(
                            valence_service_utils::denoms::UncheckedDenom::Native(
                                "NTRN".to_string(),
                            ),
                            "|account_id|:3".to_string(),
                            Some(Uint128::from(1_000_000u128)),
                            None,
                        ),
                    ],
                }),
                addr: None,
            },
        );

        config.links.insert(
            1,
            Link {
                input_accounts_id: vec![1],
                output_accounts_id: vec![2, 3],
                service_id: 1,
            },
        );

        init_workflow(config).await;

        // match timeout(Duration::from_secs(60), ).await {
        //     Ok(_) => println!("Workflow initialization completed successfully"),
        //     Err(_) => println!("Workflow initialization timed out after 60 seconds"),
        // }
    }
}
