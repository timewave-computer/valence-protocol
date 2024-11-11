#[cfg(test)]
mod test {
    use cosmwasm_std::{from_json, to_json_binary, Uint128};
    use cw_ownable::Expiration;
    use std::collections::BTreeMap;

    use crate::{
        account::{AccountInfo, AccountType},
        config::GLOBAL_CONFIG,
        domain::Domain,
        library::{LibraryConfig, LibraryInfo},
        program_config::{Link, ProgramConfig},
    };
    use serde_json_any_key::MapIterToJson;
    use valence_authorization_utils::{
        action::AtomicAction,
        authorization::{
            ActionsConfig, AtomicActionsConfig, AuthorizationDuration, AuthorizationInfo,
            AuthorizationModeInfo,
        },
        authorization_message::{Message, MessageDetails, MessageType},
    };
    use valence_library_utils::{denoms::UncheckedDenom, LibraryAccountType};

    /// test to make sure on config is parsed correctlly.
    /// MUST fix this test before handling other tests, config is part of the context we use, if we can't generate it successfully
    /// probably means other tests are also failing because of it.
    #[tokio::test]
    async fn test_config() {
        let _config = &GLOBAL_CONFIG.lock().await.general;
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
        let config =
            LibraryConfig::ValenceForwarderLibrary(valence_forwarder_library::msg::LibraryConfig {
                input_addr: "|account_id|:1".into(),
                output_addr: "|account_id|:2".into(),
                forwarding_configs: vec![
                    valence_forwarder_library::msg::UncheckedForwardingConfig {
                        denom: UncheckedDenom::Native("untrn".to_string()),
                        max_amount: Uint128::new(100),
                    },
                ],
                forwarding_constraints: valence_forwarder_library::msg::ForwardingConstraints::new(
                    None,
                ),
            });

        let account_ids = config.get_account_ids().unwrap();
        println!("{account_ids:?}");
    }

    #[ignore = "internal test"]
    #[test]
    fn test_serialize_program() {}

    #[test]
    fn test_serialize() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
        struct Test {
            test: LibraryAccountType,
        }

        let t = Test {
            test: LibraryAccountType::AccountId(1),
        };

        // let mut json = serde_json::to_string(&t).unwrap();
        let mut json = serde_json::to_string(&t).unwrap();
        println!("{json:?}");

        json = json.replace("|account_id|\":1", "library_account_addr\":\"cosmos1\"");
        println!("{json:?}");

        let back_struct = serde_json::from_str::<Test>(&json).unwrap();
        println!("{back_struct:?}");

        let mut splits: BTreeMap<LibraryAccountType, Uint128> = BTreeMap::new();
        splits.insert(LibraryAccountType::AccountId(2), 100_u128.into());
        splits.insert(LibraryAccountType::AccountId(3), 200_u128.into());
        // let to_vec = splits;
        let json = splits.to_json_map().unwrap();
        // let json = serde_json::to_string(&splits).unwrap();
        println!("{json:?}");
    }

    #[ignore = "internal test"]
    #[tokio::test]
    async fn test_parsing() {
        let json_string =
            "{\"input_addr\": {\"|account_id|\": 1}, \"output_addr\": {\"|account_id|\": 2}}";
        let config = serde_json::from_str::<valence_forwarder_library::msg::LibraryConfigUpdate>(
            json_string,
        )
        .unwrap();
        println!("{:#?}", config);
    }

    #[ignore = "internal test"]
    #[tokio::test]
    async fn test_full_program() {
        // let subscriber = tracing_subscriber::fmt()
        //     .with_max_level(tracing::Level::DEBUG)
        //     .with_test_writer()
        //     .with_span_events(FmtSpan::CLOSE)
        //     .finish();
        // tracing::subscriber::set_global_default(subscriber)
        //     .expect("setting default subscriber failed");

        // let c: Config = ConfigHelper::builder()
        //     .add_source(
        //         glob::glob("conf/*")
        //             .unwrap()
        //             .filter_map(|path| {
        //                 let p = path.unwrap();
        //                 println!("Path: {:?}", p);

        //                 if p.is_dir() {
        //                     None
        //                 } else {
        //                     Some(File::from(p))
        //                 }
        //             })
        //             .collect::<Vec<_>>(),
        //     )
        //     .add_source(
        //         glob::glob("conf/**/*")
        //             .unwrap()
        //             .filter_map(|path| {
        //                 let p = path.unwrap();
        //                 if p.is_dir() {
        //                     None
        //                 } else {
        //                     Some(File::from(p))
        //                 }
        //             })
        //             .collect::<Vec<_>>(),
        //     )
        //     .build()
        //     .unwrap()
        //     .try_deserialize()
        //     .unwrap();

        // *GLOBAL_CONFIG.lock().await = c;

        let neutron_domain = Domain::CosmosCosmwasm("neutron".to_string());

        let mut config = ProgramConfig {
            owner: "neutron1tl0w0djc5y53aqfr60a794f02drwktpujm5xxe".to_string(),
            ..Default::default()
        };

        config.accounts.insert(
            0,
            AccountInfo {
                name: "test_1".to_string(),
                ty: AccountType::Base { admin: None },
                domain: neutron_domain.clone(),
                addr: None,
            },
        );
        config.accounts.insert(
            1,
            AccountInfo {
                name: "test_2".to_string(),
                ty: AccountType::Base { admin: None },
                domain: neutron_domain.clone(),
                addr: None,
            },
        );

        config.libraries.insert(
            0,
            LibraryInfo {
                name: "test_forwarder".to_string(),
                domain: neutron_domain.clone(),
                config: LibraryConfig::ValenceForwarderLibrary(
                    valence_forwarder_library::msg::LibraryConfig {
                        input_addr: LibraryAccountType::AccountId(1),
                        output_addr: LibraryAccountType::AccountId(2),
                        forwarding_configs: vec![
                            valence_forwarder_library::msg::UncheckedForwardingConfig {
                                denom: UncheckedDenom::Native("untrn".to_string()),
                                max_amount: Uint128::new(100),
                            },
                        ],
                        forwarding_constraints:
                            valence_forwarder_library::msg::ForwardingConstraints::new(None),
                    },
                ),
                addr: None,
            },
        );

        config.links.insert(
            0,
            Link {
                input_accounts_id: vec![0],
                output_accounts_id: vec![1],
                library_id: 1,
            },
        );

        config.authorizations.insert(
            0,
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
                        contract_address: LibraryAccountType::LibraryId(1),
                    }],
                    retry_logic: None,
                }),
                priority: None,
            },
        );

        // config.authorization_data.set_processor_bridge_addr(Domain::CosmosCosmwasm("neutron".to_string()), "sdf".to_string());

        // let b = to_json_binary(&config).unwrap();
        // println!("{:#?}", b);

        // init_program(&mut config).await.unwrap();

        // Make sure we have a config in place
        let lib = config.libraries.first_key_value().unwrap().1.config.clone();
        assert_ne!(lib, LibraryConfig::None);

        let binary = to_json_binary(&config).unwrap();
        let program_config = from_json::<ProgramConfig>(&binary).unwrap();

        // After parsing, workflow config should have no library config
        let lib = program_config
            .libraries
            .first_key_value()
            .unwrap()
            .1
            .config
            .clone();
        assert_eq!(lib, LibraryConfig::None);

        // match timeout(Duration::from_secs(60), ).await {
        //     Ok(_) => println!("Program initialization completed successfully"),
        //     Err(_) => println!("Program initialization timed out after 60 seconds"),
        // }
    }
}
