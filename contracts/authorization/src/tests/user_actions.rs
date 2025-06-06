use base64::prelude::*;
use cosmwasm_std::{Binary, Coin, Timestamp, Uint128};
use cw_utils::Expiration;
use neutron_test_tube::{Account, Module, Wasm};
use serde_json::json;
use sp1_sdk::{HashableKey, SP1VerifyingKey};
use sp1_verifier::{Groth16Verifier, GROTH16_VK_BYTES};
use valence_authorization_utils::{
    authorization::{AuthorizationDuration, AuthorizationModeInfo, PermissionTypeInfo},
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder, JsonBuilder},
    msg::{ExecuteMsg, PermissionedMsg, PermissionlessMsg, ProcessorMessage, QueryMsg},
    zk_authorization::{ZkAuthorization, ZkAuthorizationInfo},
};

use crate::{
    contract::build_tokenfactory_denom,
    error::{AuthorizationErrorReason, ContractError, MessageErrorReason, UnauthorizedReason},
    tests::helpers::wait_for_height,
};

use super::{
    builders::NeutronTestAppBuilder,
    helpers::{
        instantiate_and_set_verification_gateway,
        store_and_instantiate_authorization_with_processor_contract,
    },
};

#[test]
fn disabled() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (contract_addr, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.owner_accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    // We'll create a generic permissionless authorization
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(AtomicFunctionBuilder::new().build())
                .build(),
        )
        .build()];

    // Create and disable it
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::DisableAuthorization {
            label: "permissionless".to_string(),
        }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Trying to execute this authorization should fail because it's not enabled
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotEnabled {})
            .to_string()
            .as_str()
    ));

    // Trying to execute an authorization that doesn't exist should also fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "non_existent".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Authorization(AuthorizationErrorReason::DoesNotExist(
            "non_existent".to_string()
        ))
        .to_string()
        .as_str()
    ));
}

#[test]
fn invalid_time() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (contract_addr, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.owner_accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    let current_time = setup.app.get_block_time_seconds() as u64;

    // We'll create a permissioned authorization that will be valid in the future
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissioned")
        .with_not_before(Expiration::AtTime(Timestamp::from_seconds(
            current_time + 1000,
        )))
        .with_duration(AuthorizationDuration::Seconds(1500))
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![setup.owner_addr.to_string()]),
        ))
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(AtomicFunctionBuilder::new().build())
                .build(),
        )
        .build()];

    // Create it
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Trying to execute this authorization should fail because start time hasn't been reached yet
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotActiveYet {})
            .to_string()
            .as_str()
    ));

    // Let's increase the time
    setup.app.increase_time(1000);

    // Now the time validation should pass but the authorization should fail because user doesn't have permission
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));

    // Let's increase the time to expire it
    setup.app.increase_time(501);

    // Now the time validation should fail again
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::Expired {})
            .to_string()
            .as_str()
    ));

    // Let's do it with blocks now
    let current_height = setup.app.get_block_height() as u64;
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissioned2")
        .with_not_before(Expiration::AtHeight(current_height + 10))
        .with_duration(AuthorizationDuration::Blocks(15))
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![setup.owner_addr.to_string()]),
        ))
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(AtomicFunctionBuilder::new().build())
                .build(),
        )
        .build()];

    // Create it
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Trying to execute this authorization should fail because start time hasn't been reached yet
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned2".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotActiveYet {})
            .to_string()
            .as_str()
    ));

    wait_for_height(&setup.app, current_height + 10);

    // Now the time validation should pass but the authorization should fail because user doesn't have permission
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned2".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));

    wait_for_height(&setup.app, current_height + 20);

    // Now the time validation should fail again because authorization is expired
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned2".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::Expired {})
            .to_string()
            .as_str()
    ));
}

#[test]
fn invalid_permission() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (contract_addr, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.owner_accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    // We'll create a couple permissioned authorizations
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_mode(AuthorizationModeInfo::Permissioned(
                PermissionTypeInfo::WithoutCallLimit(vec![setup.owner_addr.to_string()]),
            ))
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(AtomicFunctionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-with-limit")
            .with_mode(AuthorizationModeInfo::Permissioned(
                PermissionTypeInfo::WithCallLimit(vec![(
                    setup.user_accounts[0].address().to_string(),
                    Uint128::new(10),
                )]),
            ))
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(AtomicFunctionBuilder::new().build())
                    .build(),
            )
            .build(),
    ];

    // Create them
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Trying to execute them will give us permission errors
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));

    // Even though the user has the token, it's not enough to execute the action, he needs to send it
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-with-limit".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotAllowed {})
            .to_string()
            .as_str()
    ));

    let permission_token = build_tokenfactory_denom(&contract_addr, "permissioned-with-limit");

    // Sending more than 1 token should also fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-with-limit".to_string(),
                messages: vec![ProcessorMessage::CosmwasmExecuteMsg {
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[Coin::new(Uint128::new(2), permission_token.clone())],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::RequiresOneToken {})
            .to_string()
            .as_str()
    ));
}

#[test]
fn invalid_messages() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (contract_addr, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.owner_accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );

    // Let's create several permissionless authorizations to validate the messages against
    let authorizations = vec![
        // No param restrictions
        AuthorizationBuilder::new()
            .with_label("no-restrictions")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "execute_method".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("with-restrictions")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "execute_method".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "execute_method".to_string(),
                                            "key1".to_string(),
                                            "key2".to_string(),
                                        ]),
                                        ParamRestriction::CannotBeIncluded(vec![
                                            "execute_method".to_string(),
                                            "key3".to_string(),
                                            "key4".to_string(),
                                        ]),
                                        ParamRestriction::MustBeValue(
                                            vec![
                                                "execute_method".to_string(),
                                                "key5".to_string(),
                                                "key6".to_string(),
                                            ],
                                            Binary::from(
                                                serde_json::to_vec(&json!([1, 2, 3])).unwrap(),
                                            ),
                                        ),
                                        ParamRestriction::MustBeValue(
                                            vec!["execute_method".to_string(), "key7".to_string()],
                                            Binary::from(serde_json::to_vec(&json!(100)).unwrap()),
                                        ),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
    ];

    // Create all of them
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Let's try to execute an authorization sending more messages than expected
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![
                    ProcessorMessage::CosmwasmExecuteMsg {
                        msg: Binary::default(),
                    },
                    ProcessorMessage::CosmwasmExecuteMsg {
                        msg: Binary::default(),
                    },
                ],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidAmount {})
            .to_string()
            .as_str()
    ));

    // If we try to execute an authorization sending different messages types than expected, it should fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![ProcessorMessage::CosmwasmMigrateMsg {
                    code_id: 40,
                    msg: Binary::default(),
                }],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidType {})
            .to_string()
            .as_str()
    ));

    // If we try to execute the authorization with something that cannot be parsed into a json, it should fail
    let message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(b"This is not JSON"),
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![message],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains("Invalid JSON passed"));

    // If we try to execute the authorization with a json that has multiple top keys, it should fail
    let message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(serde_json::to_vec(&json!({"key1": "value", "key2": "value"})).unwrap()),
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![message],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidStructure {})
            .to_string()
            .as_str()
    ));

    // If we try to execute the authorization with a json that has the wrong key, it should fail
    let message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(
            serde_json::to_vec(&JsonBuilder::new().main("wrong_key").build()).unwrap(),
        ),
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![message],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::DoesNotMatch {})
            .to_string()
            .as_str()
    ));

    // If we try to execute the authorization with jsons that don't match the restriction they should fail

    // Doesn't have key1.key2
    let message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(
            serde_json::to_vec(
                &JsonBuilder::new()
                    .main("execute_method")
                    .add("key7.key8", json!("value"))
                    .build(),
            )
            .unwrap(),
        ),
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "with-restrictions".to_string(),
                messages: vec![message],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidMessageParams {})
            .to_string()
            .as_str()
    ));

    // Has key1.key2 but also has key3.key4 which is not allowed
    let message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(
            serde_json::to_vec(
                &JsonBuilder::new()
                    .main("execute_method")
                    .add("key1.key2", json!("value"))
                    .add("key3.key4", json!("value"))
                    .build(),
            )
            .unwrap(),
        ),
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "with-restrictions".to_string(),
                messages: vec![message],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidMessageParams {})
            .to_string()
            .as_str()
    ));

    // Has key1.key and doesn't have key3.key4 but key5.key6 and key7 doesn't have the values specified
    let message = ProcessorMessage::CosmwasmExecuteMsg {
        msg: Binary::from(
            serde_json::to_vec(
                &JsonBuilder::new()
                    .main("execute_method")
                    .add("key1.key2", json!("value"))
                    .add("key5.key6", json!("value"))
                    .add("key7", json!("value"))
                    .build(),
            )
            .unwrap(),
        ),
    };

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "with-restrictions".to_string(),
                messages: vec![message],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidMessageParams {})
            .to_string()
            .as_str()
    ));
}

#[test]
fn pause_and_resume_processor_using_zk_authorizations() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization, processor) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.owner_accounts[0],
        setup.owner_addr.to_string(),
        vec![],
    );

    // VK of the program that accepts 2 registry values and creates a message to pause the processor for registry 1 and
    // creates a message to resume it for registry 2
    let program_vk = "BYX3ESvOAQq9cdkc9IPbbYXpg2qdHHgX5xNkEiM0TlJUvSAAU4EVAprBNkOrsRsoMlyiE/qesXJCKaQi59g8Lxwb5k0upNZO/WR5UfCodBbxY6MSiVl5SlBHfykCAAAAAAAAAAcAAAAAAAAAUHJvZ3JhbRMAAAAAAAAAAQAAAA4AAAAAAAAAAAAIAAAAAAAEAAAAAAAAAEJ5dGUQAAAAAAAAAAEAAAALAAAAAAAAAAAAAQAAAAAAAgAAAAAAAAAEAAAAAAAAAEJ5dGUBAAAAAAAAAAcAAAAAAAAAUHJvZ3JhbQAAAAAAAAAA";
    let full_proof_pause = "2gFcRWJhZ25SaUI1VUh5UXgzc3dRY0U1WGRvRFZ3Z1pad1Z2T0szdW1xK3h0b1NEWTRhRGQzdEV6eTRsTnR5b3ErWHg3ZjFYVG8yelMvOVp2MTRleklER3F4UkIrZ1hPOWhaL3dPakpmSjdTWkVMQnAxdGQ1RFlqc21pYXM0VTFkWFQzd01KY3hGS3dnY2JBOWFnUEcyRzduY2s5YVJRaEVpUTdpdnNZam9QMnZFTGZZWHdLZWhYSHk5MFMvMFlVbzZkd1RmeWpwZ1REYndhUzdjc3N3czJzNXJhekV3Y0N6YUh6OUtWUUpSTFliZXpubTV4eHJxM0RENEhNQjhrdTJMbFdyOWpnaTF0Z2gwN3hRbXlnNEU5dXloRlUwTnBFVnlRQzBxNHNwbmNqZGhZMHVkWUZWaTlxME1paXpxc1pNeVl2NjNIVnZHZzd4MEJkbXZvQ0VJS21qWElaaTA92bBBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFCN0luSmxaMmx6ZEhKNUlqb3hMQ0ppYkc5amExOXVkVzFpWlhJaU9qRXNJbVJ2YldGcGJpSTZJbTFoYVc0aUxDSmhkWFJvYjNKcGVtRjBhVzl1WDJOdmJuUnlZV04wSWpwdWRXeHNMQ0p0WlhOellXZGxJanA3SW5CaGRYTmxJanA3ZlgxOQ==";
    let full_proof_resume = "2gFcRWJhZ25SY3RxU21SZit4VVdPR2JrQTlWQU90cTRScmlMSTdBS1hPeDhmWm9HdTRuQVByRGVsajlnT3ZJVE5iaWdVQldlc1E3b3BUZTBycmc1NDJidWFZSmVoc0FKem5GdHZSRUFHVUVXeWw0OW1qbDdsNVVYZjVLZDJEa1dJd0dNQXUxcWcyOXovaHhzdEhlbk9BV0NzVkVKQ0JKRU8xSFR5N1lEVkIwRGFRajlyRWlBZ2d4cG02QU1neitkQzN5Mk9HZ3lMZDhQNzRSNFd6b0pPUG5GU0I2VUtRYjlIdGdRcHRMTXU1dlZNYjYvdUYxS3g2YnRpRTNPQnJjRXBjTmJ0YXNRU3BEaGsxZUNENkFKRmRZQjZ0M3k5eGw0K20vWGN0cGtNUSsvYUtLdWM4RUt6Rit0ZnZBODlwa0tRZjZzbGZyZ0gvaFEwUTNudkF4RmtBNFdQSzlsdlE92bRBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFCN0luSmxaMmx6ZEhKNUlqb3lMQ0ppYkc5amExOXVkVzFpWlhJaU9qRXNJbVJ2YldGcGJpSTZJbTFoYVc0aUxDSmhkWFJvYjNKcGVtRjBhVzl1WDJOdmJuUnlZV04wSWpwdWRXeHNMQ0p0WlhOellXZGxJanA3SW5KbGMzVnRaU0k2ZTMxOWZRPT0=";
    let decoded_vk = BASE64_STANDARD.decode(program_vk).unwrap();

    let sp1_vk: SP1VerifyingKey = bincode::deserialize(&decoded_vk).unwrap();
    let proof_pause = valence_coprocessor::Proof::try_from_base64(full_proof_pause).unwrap();
    let proof_resume = valence_coprocessor::Proof::try_from_base64(full_proof_resume).unwrap();

    // Sanity check that verification works
    let (proof_pause_bytes, proof_pause_inputs) = proof_pause.decode().unwrap();
    let (proof_resume_bytes, proof_resume_inputs) = proof_resume.decode().unwrap();
    // Sanity check that verification works
    Groth16Verifier::verify(
        &proof_pause_bytes,
        &proof_pause_inputs,
        &sp1_vk.bytes32(),
        &GROTH16_VK_BYTES,
    )
    .unwrap();

    instantiate_and_set_verification_gateway(
        &setup.app,
        &setup.owner_accounts[0],
        authorization.clone(),
        setup.owner_addr.to_string(),
        Binary::from(sp1_vk.bytes32().into_bytes()),
    );

    // Let's create two zk authorizations, one to pause the processor and another to resume it, pause will have registry 1 and resume will have registry 2
    let zk_authorization_pause = ZkAuthorizationInfo {
        label: "pause".to_string(),
        mode: AuthorizationModeInfo::Permissionless,
        registry: 1,
        vk: Binary::from(sp1_vk.bytes32().into_bytes()),
        validate_last_block_execution: false,
    };
    let zk_authorization_resume = ZkAuthorizationInfo {
        label: "resume".to_string(),
        mode: AuthorizationModeInfo::Permissionless,
        registry: 2,
        vk: Binary::from(sp1_vk.bytes32().into_bytes()),
        validate_last_block_execution: false,
    };
    let zk_authorizations = vec![zk_authorization_pause, zk_authorization_resume];

    wasm.execute::<ExecuteMsg>(
        &authorization,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateZkAuthorizations {
            zk_authorizations,
        }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Check that authorizations were created
    let query_authorizations = wasm
        .query::<QueryMsg, Vec<ZkAuthorization>>(
            &authorization,
            &QueryMsg::ZkAuthorizations {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(query_authorizations.len(), 2);

    // Execute the pause processor authorization
    wasm.execute::<ExecuteMsg>(
        &authorization,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::ExecuteZkAuthorization {
            label: "pause".to_string(),
            message: Binary::from(proof_pause_inputs.clone()),
            proof: Binary::from(proof_pause_bytes.clone()),
            domain_message: Binary::from(proof_pause_inputs.clone()),
            domain_proof: Binary::from(proof_pause_bytes.clone()),
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Check that the processor is paused
    let processor_config = wasm
        .query::<valence_processor_utils::msg::QueryMsg, valence_processor_utils::processor::Config>(
            &processor,
            &valence_processor_utils::msg::QueryMsg::Config {},
        )
        .unwrap();

    assert_eq!(
        processor_config.state,
        valence_processor_utils::processor::State::Paused
    );

    // Execute the resume processor authorization
    wasm.execute::<ExecuteMsg>(
        &authorization,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::ExecuteZkAuthorization {
            label: "resume".to_string(),
            message: Binary::from(proof_resume_inputs.clone()),
            proof: Binary::from(proof_resume_bytes.clone()),
            domain_message: Binary::from(proof_resume_inputs),
            domain_proof: Binary::from(proof_resume_bytes),
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Check that the processor is resumed
    let processor_config = wasm
        .query::<valence_processor_utils::msg::QueryMsg, valence_processor_utils::processor::Config>(
            &processor,
            &valence_processor_utils::msg::QueryMsg::Config {},
        )
        .unwrap();
    assert_eq!(
        processor_config.state,
        valence_processor_utils::processor::State::Active
    );

    // Check that if we disable the authorization, we can't execute it anymore
    wasm.execute::<ExecuteMsg>(
        &authorization,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::DisableAuthorization {
            label: "pause".to_string(),
        }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::ExecuteZkAuthorization {
                label: "pause".to_string(),
                message: Binary::from(proof_pause_inputs.clone()),
                proof: Binary::from(proof_pause_bytes.clone()),
                domain_message: Binary::from(proof_pause_inputs),
                domain_proof: Binary::from(proof_pause_bytes),
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Unauthorized(UnauthorizedReason::NotEnabled {})
            .to_string()
            .as_str()
    ));
}
