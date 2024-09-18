use cosmwasm_std::{Addr, Binary, Timestamp, Uint128};
use cw_utils::{Duration, Expiration};
use neutron_test_tube::{
    neutron_std::types::cosmos::bank::v1beta1::{QueryAllBalancesRequest, QueryBalanceRequest},
    Account, Bank, Module, Wasm,
};
use valence_authorization_utils::{
    action::{ActionCallback, RetryLogic, RetryTimes},
    authorization::{
        Authorization, AuthorizationDuration, AuthorizationMode, AuthorizationState,
        PermissionType, Priority,
    },
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    domain::{Domain, ExternalDomain},
    msg::{ExecuteMsg, Mint, OwnerMsg, PermissionedMsg, QueryMsg},
};
use valence_processor::error::ContractError as ProcessorContractError;
use valence_processor_utils::msg::{
    ExecuteMsg as ProcessorExecuteMsg, PermissionlessMsg as ProcessorPermissionlessMsg,
};

use crate::{
    contract::build_tokenfactory_denom,
    error::{AuthorizationErrorReason, ContractError, UnauthorizedReason},
    tests::{
        builders::{
            AtomicActionBuilder, AtomicActionsConfigBuilder, AuthorizationBuilder,
            NeutronTestAppBuilder, NonAtomicActionBuilder, NonAtomicActionsConfigBuilder,
        },
        helpers::store_and_instantiate_authorization_with_processor_contract,
    },
};

#[test]
fn contract_instantiation() {
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let subowner2 = Addr::unchecked(setup.accounts[5].address());

    // Let's instantiate with all parameters and query them to see if they are stored correctly
    let (authorization_contract, processor_address) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.user_addr.to_string(),
            vec![setup.subowner_addr.to_string(), subowner2.to_string()],
        );

    // Query current owner
    let query_owner = wasm
        .query::<QueryMsg, cw_ownable::Ownership<String>>(
            &authorization_contract,
            &QueryMsg::Ownership {},
        )
        .unwrap();

    assert_eq!(query_owner.owner.unwrap(), setup.user_addr.to_string());

    // Query subowners
    let query_subowners = wasm
        .query::<QueryMsg, Vec<Addr>>(&authorization_contract, &QueryMsg::SubOwners {})
        .unwrap();

    assert_eq!(query_subowners.len(), 2);
    assert!(query_subowners.contains(&setup.subowner_addr));
    assert!(query_subowners.contains(&subowner2));

    // Query processor
    let query_processor = wasm
        .query::<QueryMsg, Addr>(&authorization_contract, &QueryMsg::Processor {})
        .unwrap();

    assert_eq!(query_processor, Addr::unchecked(processor_address));
}

#[test]
fn transfer_ownership() {
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let new_owner = &setup.accounts[5];

    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![],
    );

    // Current owner is going to transfer ownership to new_owner
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
            new_owner: new_owner.address(),
            expiry: None,
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // New owner is going to accept the ownership
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::UpdateOwnership(cw_ownable::Action::AcceptOwnership {}),
        &[],
        new_owner,
    )
    .unwrap();

    // Check owner has been transfered
    let query_owner = wasm
        .query::<QueryMsg, cw_ownable::Ownership<String>>(
            &authorization_contract,
            &QueryMsg::Ownership {},
        )
        .unwrap();

    assert_eq!(query_owner.owner.unwrap(), new_owner.address().to_string());

    // Trying to transfer ownership again should fail because the old owner is not the owner anymore
    // Try transfering from old owner again, should fail
    let transfer_error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::UpdateOwnership(cw_ownable::Action::TransferOwnership {
                new_owner: new_owner.address(),
                expiry: None,
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(transfer_error.to_string().contains(
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str()
    ));
}

#[test]
fn add_and_remove_sub_owners() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();
    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![],
    );

    // Owner will add a subowner
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::OwnerAction(OwnerMsg::AddSubOwner {
            sub_owner: setup.subowner_addr.to_string(),
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let query_subowners = wasm
        .query::<QueryMsg, Vec<Addr>>(&authorization_contract, &QueryMsg::SubOwners {})
        .unwrap();

    assert_eq!(query_subowners.len(), 1);
    assert_eq!(query_subowners[0], setup.subowner_addr);

    // Anyone who is not the owner trying to add or remove a subowner should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::OwnerAction(OwnerMsg::AddSubOwner {
                sub_owner: setup.subowner_addr.to_string(),
            }),
            &[],
            &setup.accounts[1],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str()
    ));

    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::OwnerAction(OwnerMsg::RemoveSubOwner {
                sub_owner: setup.subowner_addr.to_string(),
            }),
            &[],
            &setup.accounts[1],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Ownership(cw_ownable::OwnershipError::NotOwner)
            .to_string()
            .as_str()
    ));

    // Owner will remove a subowner
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::OwnerAction(OwnerMsg::RemoveSubOwner {
            sub_owner: setup.subowner_addr.to_string(),
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let query_subowners = wasm
        .query::<QueryMsg, Vec<Addr>>(&authorization_contract, &QueryMsg::SubOwners {})
        .unwrap();

    assert!(query_subowners.is_empty());
}

#[test]
fn add_external_domains() {
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(7)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![],
    );

    // Owner can add external domains
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::AddExternalDomains {
            external_domains: vec![setup.external_domain.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Check that it's added
    let query_external_domains = wasm
        .query::<QueryMsg, Vec<ExternalDomain>>(
            &authorization_contract,
            &QueryMsg::ExternalDomains {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_external_domains.len(), 1);
    assert_eq!(query_external_domains[0].name, setup.external_domain.name);
}

#[test]
fn create_valid_authorizations() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);
    let bank = Bank::new(&setup.app);

    // Let's instantiate with all parameters and query them to see if they are stored correctly
    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    let valid_authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless-authorization")
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(AtomicActionBuilder::new().build())
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "method2".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "param1".to_string(),
                                            "param2".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        // This one will mint 5 tokens to subowner_addr
        AuthorizationBuilder::new()
            .with_label("permissioned-limit-authorization")
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithCallLimit(vec![(setup.subowner_addr.clone(), Uint128::new(5))]),
            ))
            .with_duration(AuthorizationDuration::Blocks(100))
            .with_max_concurrent_executions(4)
            .with_actions_config(
                NonAtomicActionsConfigBuilder::new()
                    .with_action(
                        NonAtomicActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "method".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::CannotBeIncluded(vec![
                                            "param1".to_string(),
                                            "param2".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .with_retry_logic(RetryLogic {
                                times: RetryTimes::Amount(5),
                                interval: Duration::Time(10),
                            })
                            .build(),
                    )
                    .with_action(
                        NonAtomicActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "method".to_string(),
                                    params_restrictions: Some(vec![ParamRestriction::MustBeValue(
                                        vec!["param1".to_string(), "param2".to_string()],
                                        Binary::from_base64("aGVsbG8=").unwrap(),
                                    )]),
                                },
                            })
                            .with_retry_logic(RetryLogic {
                                times: RetryTimes::Amount(10),
                                interval: Duration::Height(5),
                            })
                            .with_callback_confirmation(ActionCallback {
                                contract_address: Addr::unchecked("address"),
                                callback_message: Binary::from_base64("aGVsbG8=").unwrap(),
                            })
                            .build(),
                    )
                    .build(),
            )
            .with_priority(Priority::High)
            .build(),
        // This one will mint 1 token to subowner_addr and 1 token to user_addr
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit-authorization")
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithoutCallLimit(vec![
                    setup.subowner_addr.clone(),
                    setup.user_addr.clone(),
                ]),
            ))
            .with_duration(AuthorizationDuration::Seconds(50000000))
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_retry_logic(RetryLogic {
                        times: RetryTimes::Amount(5),
                        interval: Duration::Time(10),
                    })
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "method".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::CannotBeIncluded(vec![
                                            "param1".to_string(),
                                            "param2".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .with_action(
                        AtomicActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "method".to_string(),
                                    params_restrictions: Some(vec![ParamRestriction::MustBeValue(
                                        vec!["param1".to_string(), "param2".to_string()],
                                        Binary::from_base64("aGVsbG8=").unwrap(),
                                    )]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
    ];

    // If someone who is not the Owner or Subowner tries to create an authorization, it should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations {
                authorizations: valid_authorizations.clone(),
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));

    // Owner will create 1 and Subowner will create 2 and both will succeed
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations {
            authorizations: vec![valid_authorizations[0].clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations {
            authorizations: vec![
                valid_authorizations[1].clone(),
                valid_authorizations[2].clone(),
            ],
        }),
        &[],
        &setup.accounts[1],
    )
    .unwrap();

    // Let's query the authorizations and check if they are stored correctly
    let query_authorizations = wasm
        .query::<QueryMsg, Vec<Authorization>>(
            &authorization_contract,
            &QueryMsg::Authorizations {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_authorizations.len(), 3);
    assert!(query_authorizations
        .iter()
        .any(|a| a.label == "permissionless-authorization"
            && a.state.eq(&AuthorizationState::Enabled)));
    assert!(query_authorizations
        .iter()
        .any(|a| a.label == "permissioned-limit-authorization"
            && a.state.eq(&AuthorizationState::Enabled)));
    assert!(query_authorizations
        .iter()
        .any(|a| a.label == "permissioned-without-limit-authorization"
            && a.state.eq(&AuthorizationState::Enabled)));

    // Let's check that amount of tokens minted to subowner_addr and user_addr are correct
    let tokenfactory_denom_permissioned_with_limit =
        build_tokenfactory_denom(&authorization_contract, "permissioned-limit-authorization");
    let tokenfactory_denom_permissioned_without_limit = build_tokenfactory_denom(
        &authorization_contract,
        "permissioned-without-limit-authorization",
    );

    let subowner_balance = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.subowner_addr.to_string(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    let user_balance = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.user_addr.to_string(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    // Neutron and the two token factory tokens
    assert_eq!(subowner_balance.balances.len(), 3);
    // Neutron and one token factory token
    assert_eq!(user_balance.balances.len(), 2);
    // Check correct amounts were minted
    assert!(subowner_balance
        .balances
        .iter()
        .any(|b| b.denom == tokenfactory_denom_permissioned_with_limit && b.amount == "5"));

    assert!(subowner_balance
        .balances
        .iter()
        .any(|b| b.denom == tokenfactory_denom_permissioned_without_limit && b.amount == "1"));

    assert!(user_balance
        .balances
        .iter()
        .any(|b| b.denom == tokenfactory_denom_permissioned_without_limit && b.amount == "1"));

    // If we try to create an authorization with the same label again, it should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations {
                authorizations: valid_authorizations,
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Authorization(AuthorizationErrorReason::LabelAlreadyExists(
            "permissionless-authorization".to_string()
        ))
        .to_string()
        .as_str()
    ));
}

#[test]
fn create_invalid_authorizations() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    // Let's instantiate with all parameters and query them to see if they are stored correctly
    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![],
    );

    // Invalid authorizations and the errors we are supposed to get for each one
    let invalid_authorizations = vec![
        (
            AuthorizationBuilder::new().build(),
            ContractError::Authorization(AuthorizationErrorReason::NoActions {}),
        ),
        (
            AuthorizationBuilder::new()
                .with_label("")
                .with_actions_config(
                    AtomicActionsConfigBuilder::new()
                        .with_action(AtomicActionBuilder::new().build())
                        .build(),
                )
                .build(),
            ContractError::Authorization(AuthorizationErrorReason::EmptyLabel {}),
        ),
        (
            AuthorizationBuilder::new()
                .with_label("label")
                .with_actions_config(
                    AtomicActionsConfigBuilder::new()
                        .with_action(
                            AtomicActionBuilder::new()
                                .with_domain(Domain::External("ethereum".to_string()))
                                .build(),
                        )
                        .build(),
                )
                .build(),
            ContractError::DomainIsNotRegistered("ethereum".to_string()),
        ),
        (
            AuthorizationBuilder::new()
                .with_actions_config(
                    AtomicActionsConfigBuilder::new()
                        .with_action(AtomicActionBuilder::new().with_domain(Domain::Main).build())
                        .with_action(
                            AtomicActionBuilder::new()
                                .with_domain(Domain::External("osmosis".to_string()))
                                .build(),
                        )
                        .build(),
                )
                .build(),
            ContractError::Authorization(AuthorizationErrorReason::DifferentActionDomains {}),
        ),
        (
            AuthorizationBuilder::new()
                .with_actions_config(
                    AtomicActionsConfigBuilder::new()
                        .with_action(AtomicActionBuilder::new().build())
                        .build(),
                )
                .with_priority(Priority::High)
                .build(),
            ContractError::Authorization(
                AuthorizationErrorReason::PermissionlessWithHighPriority {},
            ),
        ),
    ];

    for (authorization, error) in invalid_authorizations {
        let execute_error = wasm
            .execute::<ExecuteMsg>(
                &authorization_contract,
                &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations {
                    authorizations: vec![authorization],
                }),
                &[],
                &setup.accounts[0],
            )
            .unwrap_err();

        assert!(execute_error
            .to_string()
            .contains(error.to_string().as_str()));
    }
}

#[test]
fn modify_authorization() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    let authorization = AuthorizationBuilder::new()
        .with_mode(AuthorizationMode::Permissioned(
            PermissionType::WithoutCallLimit(vec![setup.user_addr]),
        ))
        .with_actions_config(
            AtomicActionsConfigBuilder::new()
                .with_action(AtomicActionBuilder::new().build())
                .build(),
        )
        .build();

    // Let's create the authorization
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations {
            authorizations: vec![authorization.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Let's modify the authorization, both the owner and the subowner can modify it
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::ModifyAuthorization {
            label: "authorization".to_string(),
            not_before: Some(Expiration::AtTime(Timestamp::from_seconds(100))),
            expiration: Some(Expiration::AtHeight(50)),
            max_concurrent_executions: None,
            priority: None,
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Query to verify it changed
    let query_authorizations = wasm
        .query::<QueryMsg, Vec<Authorization>>(
            &authorization_contract,
            &QueryMsg::Authorizations {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_authorizations[0].expiration, Expiration::AtHeight(50));

    // Let's change the other fields
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::ModifyAuthorization {
            label: "authorization".to_string(),
            not_before: None,
            expiration: None,
            max_concurrent_executions: Some(5),
            priority: Some(Priority::High),
        }),
        &[],
        &setup.accounts[1],
    )
    .unwrap();

    // Query to verify it changed
    let query_authorizations = wasm
        .query::<QueryMsg, Vec<Authorization>>(
            &authorization_contract,
            &QueryMsg::Authorizations {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_authorizations[0].max_concurrent_executions, 5);
    assert_eq!(query_authorizations[0].priority, Priority::High);

    // If we try to execute as a user instead of owner it should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::ModifyAuthorization {
                label: "authorization".to_string(),
                not_before: None,
                expiration: None,
                max_concurrent_executions: None,
                priority: Some(Priority::Medium),
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));

    // Try to modify an authorization that doesn't exist should also fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::ModifyAuthorization {
                label: "non-existing-label".to_string(),
                not_before: None,
                expiration: None,
                max_concurrent_executions: None,
                priority: Some(Priority::Medium),
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(
            "non-existing-label".to_string()
        ))
        .to_string()
        .as_str()
    ));

    // Disabling an authorization should also work
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::DisableAuthorization {
            label: "authorization".to_string(),
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Query to verify it was disabled
    let query_authorizations = wasm
        .query::<QueryMsg, Vec<Authorization>>(
            &authorization_contract,
            &QueryMsg::Authorizations {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_authorizations[0].state, AuthorizationState::Disabled);

    // Let's enable it again
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::EnableAuthorization {
            label: "authorization".to_string(),
        }),
        &[],
        &setup.accounts[1],
    )
    .unwrap();

    // Query to verify it was enabled again
    let query_authorizations = wasm
        .query::<QueryMsg, Vec<Authorization>>(
            &authorization_contract,
            &QueryMsg::Authorizations {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_authorizations[0].state, AuthorizationState::Enabled);

    // Trying to disable or enable as user should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::DisableAuthorization {
                label: "authorization".to_string(),
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));
}

#[test]
fn mint_authorizations() {
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);
    let bank = Bank::new(&setup.app);

    let user2 = &setup.accounts[5];
    let user2_addr = Addr::unchecked(user2.address());

    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(AtomicActionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-limit")
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithCallLimit(vec![(setup.user_addr.clone(), Uint128::new(10))]),
            ))
            .with_duration(AuthorizationDuration::Blocks(50000))
            .with_max_concurrent_executions(4)
            .with_actions_config(
                AtomicActionsConfigBuilder::new()
                    .with_action(AtomicActionBuilder::new().build())
                    .build(),
            )
            .build(),
    ];

    // Let's create the authorization
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // If we try to mint authorizations for the permissionless one, it should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::MintAuthorizations {
                label: "permissionless".to_string(),
                mints: vec![Mint {
                    address: setup.user_addr.clone(),
                    amount: Uint128::new(1),
                }],
            }),
            &[],
            &setup.accounts[1],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Authorization(AuthorizationErrorReason::CantMintForPermissionless {})
            .to_string()
            .as_str()
    ));

    // Check balances before minting
    let tokenfactory_denom_permissioned_limit =
        build_tokenfactory_denom(&authorization_contract, "permissioned-limit");

    let user1_balance_before = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_addr.to_string(),
            denom: tokenfactory_denom_permissioned_limit.clone(),
        })
        .unwrap();

    let user2_balance_before = bank
        .query_balance(&QueryBalanceRequest {
            address: user2_addr.to_string(),
            denom: tokenfactory_denom_permissioned_limit.clone(),
        })
        .unwrap();

    // What we minted during creation
    assert_eq!(user1_balance_before.balance.unwrap().amount, "10");
    assert_eq!(user2_balance_before.balance.unwrap().amount, "0");

    // Let's mint an extra permissioned token to user1 and some additional ones for user2
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::MintAuthorizations {
            label: "permissioned-limit".to_string(),
            mints: vec![
                Mint {
                    address: setup.user_addr.clone(),
                    amount: Uint128::new(1),
                },
                Mint {
                    address: user2_addr.clone(),
                    amount: Uint128::new(5),
                },
            ],
        }),
        &[],
        &setup.accounts[1],
    )
    .unwrap();

    // Check balances after minting
    let user1_balance_after = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_addr.to_string(),
            denom: tokenfactory_denom_permissioned_limit.clone(),
        })
        .unwrap();

    let user2_balance_after = bank
        .query_balance(&QueryBalanceRequest {
            address: user2_addr.to_string(),
            denom: tokenfactory_denom_permissioned_limit.clone(),
        })
        .unwrap();

    // What we minted during creation + 1
    assert_eq!(user1_balance_after.balance.unwrap().amount, "11");
    // What we minted during creation + 5
    assert_eq!(user2_balance_after.balance.unwrap().amount, "5");

    // Trying to mint as not owner or subowner should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::MintAuthorizations {
                label: "permissioned-limit".to_string(),
                mints: vec![Mint {
                    address: setup.user_addr.clone(),
                    amount: Uint128::new(1),
                }],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));
}

#[test]
fn pausing_and_resuming_processor() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![],
        );

    // Let's pause the processor
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::PauseProcessor {
            domain: Domain::Main,
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Ticking the processor will fail because processor is paused
    let error = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ProcessorContractError::ProcessorPaused {}
            .to_string()
            .as_str()
    ));

    // Let's resume the processor
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::ResumeProcessor {
            domain: Domain::Main,
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Ticking the processor now will fail because there are no messages to process
    let error = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ProcessorContractError::NoMessagesToProcess {}
            .to_string()
            .as_str()
    ));
}
