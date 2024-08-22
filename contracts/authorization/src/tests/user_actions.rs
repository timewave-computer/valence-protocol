use cosmwasm_std::{Binary, Uint128};
use neutron_test_tube::{neutron_std::types::cosmos::base::v1beta1::Coin, Module, Wasm};
use serde_json::json;
use valence_authorization_utils::{
    authorization::{AuthorizationDuration, AuthorizationMode, PermissionType, StartTime},
    message::{Message, MessageDetails, MessageType, ParamRestriction},
};

use crate::{
    contract::build_tokenfactory_denom,
    error::{AuthorizationErrorReason, ContractError, MessageErrorReason, UnauthorizedReason},
    msg::{ExecuteMsg, SubOwnerMsg, UserMsg},
    tests::{builders::JsonBuilder, helpers::wait_for_height},
};

use super::{
    builders::{ActionBatchBuilder, ActionBuilder, AuthorizationBuilder, NeutronTestAppBuilder},
    helpers::store_and_instantiate_authorization_contract,
};

#[test]
fn disabled() {
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let contract_addr = store_and_instantiate_authorization_contract(
        &wasm,
        &setup.accounts[0],
        None,
        vec![setup.subowner_addr.clone()],
        setup.processor_addr.clone(),
        vec![setup.external_domain.clone()],
    );

    // We'll create a generic permissionless authorization
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_action(ActionBuilder::new().build())
                .build(),
        )
        .build()];

    // Create and disable it
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::SubOwnerAction(SubOwnerMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::SubOwnerAction(SubOwnerMsg::DisableAuthorization {
            label: "permissionless".to_string(),
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Trying to execute this authorization should fail because it's not enabled
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![Binary::default()],
            }),
            &[],
            &setup.accounts[2],
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
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "non_existent".to_string(),
                messages: vec![Binary::default()],
            }),
            &[],
            &setup.accounts[2],
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
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let contract_addr = store_and_instantiate_authorization_contract(
        &wasm,
        &setup.accounts[0],
        None,
        vec![setup.subowner_addr.clone()],
        setup.processor_addr.clone(),
        vec![setup.external_domain.clone()],
    );

    let current_time = setup.app.get_block_time_seconds() as u64;

    // We'll create a permissioned authorization that will be valid in the future
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissioned")
        .with_start_time(StartTime::AtTime(current_time + 1000))
        .with_duration(AuthorizationDuration::Seconds(1500))
        .with_mode(AuthorizationMode::Permissioned(
            PermissionType::WithoutCallLimit(vec![setup.owner_addr.clone()]),
        ))
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_action(ActionBuilder::new().build())
                .build(),
        )
        .build()];

    // Create it
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::SubOwnerAction(SubOwnerMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Trying to execute this authorization should fail because start time hasn't been reached yet
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned".to_string(),
                messages: vec![Binary::default()],
            }),
            &[],
            &setup.accounts[2],
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
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned".to_string(),
                messages: vec![Binary::default()],
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

    // Let's increase the time to expire it
    setup.app.increase_time(501);

    // Now the time validation should fail again
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned".to_string(),
                messages: vec![Binary::default()],
            }),
            &[],
            &setup.accounts[2],
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
        .with_start_time(StartTime::AtHeight(current_height + 10))
        .with_duration(AuthorizationDuration::Blocks(15))
        .with_mode(AuthorizationMode::Permissioned(
            PermissionType::WithoutCallLimit(vec![setup.owner_addr.clone()]),
        ))
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_action(ActionBuilder::new().build())
                .build(),
        )
        .build()];

    // Create it
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::SubOwnerAction(SubOwnerMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Trying to execute this authorization should fail because start time hasn't been reached yet
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned2".to_string(),
                messages: vec![Binary::default()],
            }),
            &[],
            &setup.accounts[2],
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
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned2".to_string(),
                messages: vec![Binary::default()],
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

    wait_for_height(&setup.app, current_height + 20);

    // Now the time validation should fail again because authorization is expired
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned2".to_string(),
                messages: vec![Binary::default()],
            }),
            &[],
            &setup.accounts[2],
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
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let contract_addr = store_and_instantiate_authorization_contract(
        &wasm,
        &setup.accounts[0],
        None,
        vec![setup.subowner_addr.clone()],
        setup.processor_addr.clone(),
        vec![setup.external_domain.clone()],
    );

    // We'll create a couple permissioned authorizations
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithoutCallLimit(vec![setup.owner_addr.clone()]),
            ))
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(ActionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-with-limit")
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithCallLimit(vec![(setup.user_addr.clone(), Uint128::new(10))]),
            ))
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(ActionBuilder::new().build())
                    .build(),
            )
            .build(),
    ];

    // Create them
    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::SubOwnerAction(SubOwnerMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Trying to execute them will give us permission errors
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![Binary::default()],
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

    // Even though the user has the token, it's not enough to execute the action, he needs to send it
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned-with-limit".to_string(),
                messages: vec![Binary::default()],
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

    let permission_token = build_tokenfactory_denom(&contract_addr, "permissioned-with-limit");

    // Sending more than 1 token should also fail
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "permissioned-with-limit".to_string(),
                messages: vec![Binary::default()],
            }),
            &[Coin {
                denom: permission_token.clone(),
                amount: "2".to_string(),
            }],
            &setup.accounts[2],
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
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

    let wasm = Wasm::new(&setup.app);

    let contract_addr = store_and_instantiate_authorization_contract(
        &wasm,
        &setup.accounts[0],
        None,
        vec![setup.subowner_addr.clone()],
        setup.processor_addr.clone(),
        vec![setup.external_domain.clone()],
    );

    // Let's create several permissionless authorizations to validate the messages against
    let authorizations = vec![
        // No param restrictions
        AuthorizationBuilder::new()
            .with_label("no-restrictions")
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(
                        ActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::ExecuteMsg,
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
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(
                        ActionBuilder::new()
                            .with_message_details(MessageDetails {
                                message_type: MessageType::ExecuteMsg,
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
        &ExecuteMsg::SubOwnerAction(SubOwnerMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Let's try to execute an authorization sending more messages than expected
    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![Binary::default(), Binary::default()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidAmount {})
            .to_string()
            .as_str()
    ));

    // If we try to execute the authorization with something that cannot be parsed into a json, it should fail
    let message = Binary::from(b"This is not JSON");

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![message],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains("Invalid JSON passed"));

    // If we try to execute the authorization with a json that has multiple top keys, it should fail
    let message =
        Binary::from(serde_json::to_vec(&json!({"key1": "value", "key2": "value"})).unwrap());

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![message],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidStructure {})
            .to_string()
            .as_str()
    ));

    // If we try to execute the authorization with a json that has the wrong key, it should fail
    let message = JsonBuilder::new().main("wrong_key").build();

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "no-restrictions".to_string(),
                messages: vec![Binary::from(serde_json::to_vec(&message).unwrap())],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::DoesNotMatch {})
            .to_string()
            .as_str()
    ));

    // If we try to execute the authorization with jsons that don't match the restriction they should fail

    // Doesn't have key1.key2
    let message = JsonBuilder::new()
        .main("execute_method")
        .add("key7.key8", json!("value"))
        .build();

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "with-restrictions".to_string(),
                messages: vec![Binary::from(serde_json::to_vec(&message).unwrap())],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidMessageParams {})
            .to_string()
            .as_str()
    ));

    // Has key1.key2 but also has key3.key4 which is not allowed
    let message = JsonBuilder::new()
        .main("execute_method")
        .add("key1.key2", json!("value"))
        .add("key3.key4", json!("value"))
        .build();

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "with-restrictions".to_string(),
                messages: vec![Binary::from(serde_json::to_vec(&message).unwrap())],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidMessageParams {})
            .to_string()
            .as_str()
    ));

    // Has key1.key and doesn't have key3.key4 but key5.key6 and key7 doesn't have the values specified
    let message = JsonBuilder::new()
        .main("execute_method")
        .add("key1.key2", json!("value"))
        .add("key5.key6", json!("value"))
        .add("key7", json!("value"))
        .build();

    let error = wasm
        .execute::<ExecuteMsg>(
            &contract_addr,
            &ExecuteMsg::UserAction(UserMsg::SendMsgs {
                label: "with-restrictions".to_string(),
                messages: vec![Binary::from(serde_json::to_vec(&message).unwrap())],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Message(MessageErrorReason::InvalidMessageParams {})
            .to_string()
            .as_str()
    ));

    // The message is valid, we should get no errors
    let message = JsonBuilder::new()
        .main("execute_method")
        .add("key1.key2", json!("value"))
        .add("key5.key6", json!([1, 2, 3]))
        .add("key7", json!(100))
        .build();

    println!("{:?}", message);

    wasm.execute::<ExecuteMsg>(
        &contract_addr,
        &ExecuteMsg::UserAction(UserMsg::SendMsgs {
            label: "with-restrictions".to_string(),
            messages: vec![Binary::from(serde_json::to_vec(&message).unwrap())],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();
}
