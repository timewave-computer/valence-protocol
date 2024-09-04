use cosmwasm_std::{Addr, Binary};
use cw_utils::Duration;
use neutron_test_tube::{Module, Wasm};
use valence_authorization_utils::{
    action::{ActionCallback, RetryLogic, RetryTimes},
    authorization::{ActionBatch, AuthorizationMode, ExecutionType, PermissionType, Priority},
    authorization_message::{Message, MessageDetails, MessageType},
    callback::{CallbackInfo, ExecutionResult},
    domain::Domain,
    msg::{ExecuteMsg, PermissionedMsg, PermissionlessMsg, ProcessorMessage, QueryMsg},
};
use valence_processor_utils::{msg::InternalProcessorMsg, processor::MessageBatch};

use crate::{
    error::{AuthorizationErrorReason, ContractError},
    tests::helpers::{wait_for_height, ARTIFACTS_DIR},
};
use valence_processor_utils::msg::{
    ExecuteMsg as ProcessorExecuteMsg, PermissionlessMsg as ProcessorPermissionlessMsg,
    QueryMsg as ProcessorQueryMsg,
};

use valence_processor::error::ContractError as ProcessorContractError;

use valence_test_service::msg::{
    ExecuteMsg as TestServiceExecuteMsg, QueryMsg as TestServiceQueryMsg,
};

use super::{
    builders::{
        ActionBatchBuilder, ActionBuilder, AuthorizationBuilder, JsonBuilder, NeutronTestAppBuilder,
    },
    helpers::{
        store_and_instantiate_authorization_with_processor_contract,
        store_and_instantiate_test_service,
    },
};

#[test]
fn user_enqueing_messages() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );

    // We'll create two authorization, one with high priority and one without to test the correct queueing of messages
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_max_concurrent_executions(10)
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(ActionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_max_concurrent_executions(10)
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithoutCallLimit(vec![
                    setup.subowner_addr.clone(),
                    setup.user_addr.clone(),
                ]),
            ))
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(ActionBuilder::new().build())
                    .build(),
            )
            .with_priority(Priority::High)
            .build(),
    ];

    // Let's create the authorizations
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Before sending any messages, let's verify the queues are empty
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);
    assert_eq!(query_high_prio_queue.len(), 0);

    let binary =
        Binary::from(serde_json::to_vec(&JsonBuilder::new().main("method").build()).unwrap());
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Let's enqueue a message for the medium priority queue
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // This should have enqueued one message in the medium priority queue, and none in the high priority queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);
    assert_eq!(query_med_prio_queue[0].id, 0);
    assert_eq!(query_med_prio_queue[0].msgs, vec![message.clone()]);
    assert_eq!(query_high_prio_queue.len(), 0);

    // Let's enqueue a few more and check that ids are incrementing
    for _ in 0..5 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();
    }

    // Query and check
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    for (i, batch) in query_med_prio_queue.iter().enumerate() {
        assert_eq!(batch.id, i as u64);
        assert_eq!(batch.msgs, vec![message.clone()]);
    }

    // Query with pagination and check that result is correct
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: Some(2),
                to: Some(4),
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 2);
    for (i, batch) in query_med_prio_queue.iter().enumerate() {
        assert_eq!(batch.id, i as u64 + 2);
        assert_eq!(batch.msgs, vec![message.clone()]);
    }

    // Let's add now to the high priority queue and see that it's correctly enqueued with the right id and the medium priority queue is untouched
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissioned-without-limit".to_string(),
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 6);

    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_high_prio_queue.len(), 1);
    assert_eq!(query_high_prio_queue[0].id, 6);

    // Add a few more to the high priority queue
    for _ in 0..5 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();
    }

    // Query and check
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_high_prio_queue.len(), 6);

    for (i, batch) in query_high_prio_queue.iter().enumerate() {
        assert_eq!(batch.id, i as u64 + 6);
        assert_eq!(batch.msgs, vec![message.clone()]);
    }
}

#[test]
fn max_concurrent_execution_limit() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );

    // We'll create an authorization with max concurrent executions and check that we can't queue more than that
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_max_concurrent_executions(3)
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_action(ActionBuilder::new().build())
                .build(),
        )
        .build()];

    // Let's create the authorization
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // We should be able to enqueue this 3 times
    let binary =
        Binary::from(serde_json::to_vec(&JsonBuilder::new().main("method").build()).unwrap());
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    for _ in 0..3 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();
    }

    // Now we should not be able to enqueue more
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Authorization(AuthorizationErrorReason::MaxConcurrentExecutionsReached {})
            .to_string()
            .as_str()
    ));

    // Owner should be able to enqueue without this limitation
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
            label: "permissionless".to_string(),
            queue_position: 0, // At the front
            priority: Priority::Medium,
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Let's check that the queue has the right amount of batches
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![3, 0, 1, 2]
    );
}

#[test]
fn owner_adding_and_removing_messages() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );

    // We'll create two authorization, one with high priority and one without to test the correct queueing of messages
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_max_concurrent_executions(10)
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(ActionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_max_concurrent_executions(10)
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithoutCallLimit(vec![
                    setup.subowner_addr.clone(),
                    setup.user_addr.clone(),
                ]),
            ))
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(ActionBuilder::new().build())
                    .build(),
            )
            .with_priority(Priority::High)
            .build(),
    ];

    // Let's create the authorizations
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Let's enqueue a few messages on both queues
    let binary =
        Binary::from(serde_json::to_vec(&JsonBuilder::new().main("method").build()).unwrap());
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    for _ in 0..5 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();

        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();
    }

    // Both should have 5 messages each in order
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![0, 2, 4, 6, 8]
    );
    assert_eq!(
        query_high_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![1, 3, 5, 7, 9]
    );

    // Let's add a message to the front of the medium priority queue
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
            label: "permissionless".to_string(),
            queue_position: 0,
            priority: Priority::Medium,
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Now the medium priority queue should have 6 messages with the new one at the front
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![10, 0, 2, 4, 6, 8]
    );

    // Let's insert a message in the middle and one at the end
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
            label: "permissionless".to_string(),
            queue_position: 3,
            priority: Priority::Medium,
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
            label: "permissionless".to_string(),
            queue_position: 7,
            priority: Priority::Medium,
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Now the medium priority queue should have 8 messages with the new ones at the right positions
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![10, 0, 2, 11, 4, 6, 8, 12]
    );

    // Trying to add messages outside of bounds will give an error
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
                label: "permissionless".to_string(),
                queue_position: 9,
                priority: Priority::Medium,
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains("Index out of bounds"));

    // Let's try to remove all messages from the high priority queue
    for _ in 0..5 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::EvictMsgs {
                domain: Domain::Main,
                queue_position: 0,
                priority: Priority::High,
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap();
    }

    // Now the high priority queue should be empty
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_high_prio_queue.len(), 0);

    // We should have 5 confirmed callbacks all with RemovedByOwner result
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 5);

    let expected_callbacks = [1, 3, 5, 7, 9];
    for (index, confirmed_callback) in query_confirmed_callbacks.iter().enumerate() {
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::RemovedByOwner
        );
        assert_eq!(confirmed_callback.execution_id, expected_callbacks[index]);
    }

    // Trying to remove again will return an error because the queue is empty
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::EvictMsgs {
                domain: Domain::Main,
                queue_position: 0,
                priority: Priority::High,
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains("Index out of bounds"));

    // Now that the queue is empty let's add some messages manually instead of by the user
    // We are going to add them all one by one to the front instead of the normal push back behavior
    for _ in 0..5 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
                label: "permissioned-without-limit".to_string(),
                queue_position: 0,
                priority: Priority::High,
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap();
    }

    // Now the high priority queue should have 5 messages with the new ones at the front
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(
        query_high_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![17, 16, 15, 14, 13]
    );

    // Let's try to remove from the back instead from than from the front now
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::EvictMsgs {
            domain: Domain::Main,
            queue_position: 4,
            priority: Priority::High,
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Now the high priority queue should have 4 messages with the last one removed
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(
        query_high_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![17, 16, 15, 14]
    );

    // Add a new one to the back
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::InsertMsgs {
            label: "permissioned-without-limit".to_string(),
            queue_position: 4,
            priority: Priority::High,
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Now the high priority queue should have 5 messages with the new one at the back
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(
        query_high_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![17, 16, 15, 14, 18]
    );

    // Finally lets just clean up the entire queue again from the back and see that it correctly empties
    for i in (0..5).rev() {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::EvictMsgs {
                domain: Domain::Main,
                queue_position: i,
                priority: Priority::High,
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap();
    }

    // Now the high priority queue should be empty
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_high_prio_queue.len(), 0);

    // Let's check the confirmed callbacks again
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // We removed 6 more
    assert_eq!(query_confirmed_callbacks.len(), 11);
    // We removed starting from the back
    let expected_callbacks = [1, 3, 5, 7, 9, 13, 14, 15, 16, 17, 18];
    for (index, confirmed_callback) in query_confirmed_callbacks.iter().enumerate() {
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::RemovedByOwner
        );
        assert_eq!(confirmed_callback.execution_id, expected_callbacks[index]);
    }

    // The medium queue should not have been touched during the entire process
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![10, 0, 2, 11, 4, 6, 8, 12]
    );
}

#[test]
fn invalid_msg_rejected() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // Let's create an authorization for sending a message to the test service that doesn't even exist on the contract
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_max_concurrent_executions(10)
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .build(),
                )
                .build(),
        )
        .build()];

    // Let's create the authorization
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Let's try to send an invalid message to the test service
    let binary =
        Binary::from(serde_json::to_vec(&JsonBuilder::new().main("method").build()).unwrap());
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send it, which will add it to the queue
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Confirm that we have one message in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);

    // If we tick the processor, the message will fail, the callback will be sent to the authorization contract with the right error, and will be removed from queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Was removed from queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // And the callback was sent to the authorization contract
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert!(matches!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::Rejected(_)
    ));
}

#[test]
fn queue_shifting_when_not_retriable() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // Let's create two authorizations (one atomic and one non atomic) that will always fail and see that when they fail, they are put back on the back in the queue
    // and when the retrying cooldown is not reached, they are shifted to the back of the queue
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless-atomic")
            .with_max_concurrent_executions(2)
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_error".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .with_retry_logic(RetryLogic {
                        times: RetryTimes::Indefinitely,
                        interval: Duration::Height(50), // 50 blocks between retries
                    })
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissionless-non-atomic")
            .with_max_concurrent_executions(2)
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_execution_type(ExecutionType::NonAtomic)
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_error".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .with_retry_logic(RetryLogic {
                                times: RetryTimes::Indefinitely,
                                interval: Duration::Height(50), // 50 blocks between retries
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
    ];

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Let's send 2 messages to the queue
    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillError {
            error: "fails".to_string(),
        })
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-atomic".to_string(),
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-non-atomic".to_string(),
            messages: vec![message.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Confirm that we have two messages in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 2);
    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![0, 1]
    );

    // Ticking the processor will make the first message fail and be put back at the end of the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Check there are no confirmed callbacks
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 0);

    // Confirm that we have two messages in the queue, but the first one is now at the end
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 2);
    assert_eq!(
        query_med_prio_queue
            .iter()
            .map(|batch| batch.id)
            .collect::<Vec<u64>>(),
        vec![1, 0]
    );

    // Ticking the processor again will make the first message fail and be put back at the end of the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Now the first message should be back at the beginning of the queue, and if we tick, we'll just shift the queue but not attempt to process anything
    // because retry method has not been reached
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

    assert!(response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_action_back_to_queue")));

    // Let's tick again to check that the same happens with the non atomic action
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

    assert!(response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_action_back_to_queue")));

    // Let's increase the block height enouch to trigger the retry and double check that the action is tried again
    let current_height = setup.app.get_block_height() as u64;
    wait_for_height(&setup.app, current_height + 50);

    // If we tick know we will try to execute
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

    assert!(!response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_action_back_to_queue")));

    // Same for non atomic action
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

    assert!(!response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_action_back_to_queue")));
}

#[test]
fn higher_priority_queue_is_processed_first() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create two authorizations, one with high priority and one without, and we'll enqueue two messages for both
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_max_concurrent_executions(10)
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_max_concurrent_executions(10)
            .with_mode(AuthorizationMode::Permissioned(
                PermissionType::WithoutCallLimit(vec![setup.user_addr.clone()]),
            ))
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .with_priority(Priority::High)
            .build(),
    ];

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Let's execute two of each
    for _ in 0..2 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();

        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![message.clone()],
            }),
            &[],
            &setup.accounts[2],
        )
        .unwrap();
    }

    // Let's check that the high priority queue is processed first
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // We should have process 1 message from the high priority queue, let's see that there's only 1 left
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_high_prio_queue.len(), 1);

    // Let's confirm the callback in the authorization contract as successful
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let expected_callbacks = [1];
    for (index, confirmed_callback) in query_confirmed_callbacks.iter().enumerate() {
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::Success
        );
        assert_eq!(confirmed_callback.execution_id, expected_callbacks[index]);
    }

    // Now let's tick again to process the other message in the high priority queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // We should have nothing left now
    let query_high_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::High,
            },
        )
        .unwrap();

    assert_eq!(query_high_prio_queue.len(), 0);

    // We should have two callbacks now
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let expected_callbacks = [1, 3];
    for (index, confirmed_callback) in query_confirmed_callbacks.iter().enumerate() {
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::Success
        );
        assert_eq!(confirmed_callback.execution_id, expected_callbacks[index]);
    }

    // There should be two messages left in the medium priority queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 2);

    // Let's tick twice to process them and check that the medium queue is empty and we received all callbacks
    for _ in 0..2 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();
    }

    // We should have nothing left now
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // We should have four confirmed callbacks now
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let expected_callbacks = [0, 1, 2, 3];
    for (index, confirmed_callback) in query_confirmed_callbacks.iter().enumerate() {
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::Success
        );
        assert_eq!(confirmed_callback.execution_id, expected_callbacks[index]);
    }
}

#[test]
fn retry_multi_action_atomic_batch_until_success() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create an authorization with 3 actions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_retry_logic(RetryLogic {
                    times: RetryTimes::Indefinitely,
                    interval: Duration::Time(2),
                })
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed_if_true".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary =
        Binary::from(serde_json::to_vec(&TestServiceExecuteMsg::WillSucceedIfTrue {}).unwrap());
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1.clone(), message2, message1],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Confirm that we have one batch with three messages in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);
    assert_eq!(query_med_prio_queue[0].msgs.len(), 3);

    // The batch will constantly fail because the second message will always fail
    for _ in 0..5 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

        // Check that it's still in the queue
        let query_med_prio_queue = wasm
            .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
                &processor_contract,
                &ProcessorQueryMsg::GetQueue {
                    from: None,
                    to: None,
                    priority: Priority::Medium,
                },
            )
            .unwrap();

        assert_eq!(query_med_prio_queue.len(), 1);
        setup.app.increase_time(5);
    }

    // Set the condition to true to make it succeed
    wasm.execute::<TestServiceExecuteMsg>(
        &test_service_contract,
        &TestServiceExecuteMsg::SetCondition { condition: true },
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Ticking now will make it succeed
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Confirm that we have no messages in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Confirm we got the callback
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert_eq!(query_confirmed_callbacks[0].messages.len(), 3);
    assert_eq!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::Success
    );
}

#[test]
fn retry_multi_action_non_atomic_batch_until_success() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create an authorization with 3 actions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_execution_type(ExecutionType::NonAtomic)
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_retry_logic(RetryLogic {
                            times: RetryTimes::Indefinitely,
                            interval: Duration::Time(2),
                        })
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed_if_true".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary =
        Binary::from(serde_json::to_vec(&TestServiceExecuteMsg::WillSucceedIfTrue {}).unwrap());
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1.clone(), message2, message1],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Confirm that we have one batch with three messages in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);
    assert_eq!(query_med_prio_queue[0].msgs.len(), 3);

    // Ticking the first time will make the first message succeed and re-add to the queue to move to the second message
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Check that it's back in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);

    // No matter how many times we tick, the second message will always fail and it will be re-added to the queue
    for _ in 0..5 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

        // Check that it's still in the queue
        let query_med_prio_queue = wasm
            .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
                &processor_contract,
                &ProcessorQueryMsg::GetQueue {
                    from: None,
                    to: None,
                    priority: Priority::Medium,
                },
            )
            .unwrap();

        assert_eq!(query_med_prio_queue.len(), 1);
        setup.app.increase_time(5);
    }

    // Change the condition to true to make it succeed
    wasm.execute::<TestServiceExecuteMsg>(
        &test_service_contract,
        &TestServiceExecuteMsg::SetCondition { condition: true },
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Tick again will move now to the 3rd action but not process it, just re-add it to the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Check that it's back in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);

    // Last tick will process the last message and send the callback
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Confirm that we have no messages in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Confirm we got the callback
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert_eq!(query_confirmed_callbacks[0].messages.len(), 3);
}

#[test]
fn failed_atomic_batch_after_retries() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create an authorization with 3 actions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_retry_logic(RetryLogic {
                    times: RetryTimes::Amount(5),
                    interval: Duration::Time(2),
                })
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_retry_logic(RetryLogic {
                            times: RetryTimes::Indefinitely,
                            interval: Duration::Time(2),
                        })
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_error".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillError {
            error: "failed".to_string(),
        })
        .unwrap(),
    );
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1, message2],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Trying to trigger the ExecuteAtomic entry point will fail because only the processor can call it
    let error = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::InternalProcessorAction(InternalProcessorMsg::ExecuteAtomic {
                batch: MessageBatch {
                    id: 0,
                    msgs: vec![],
                    action_batch: ActionBatch {
                        execution_type: ExecutionType::Atomic,
                        actions: vec![],
                        retry_logic: None,
                    },
                    priority: Priority::Medium,
                },
            }),
            &[],
            &setup.accounts[0],
        )
        .unwrap_err();

    assert!(error
        .to_string()
        .contains(ProcessorContractError::NotProcessor {}.to_string().as_str()));

    // Ticking 6 times (first time + retry amount) will send the callback with the error to the authorization contract
    for _ in 0..6 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

        setup.app.increase_time(5);
    }

    // Check it has been removed from the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Confirm we got the callback
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert!(matches!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::Rejected(_)
    ));
}

#[test]
fn failed_non_atomic_batch_after_retries() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create an authorization with 3 actions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_execution_type(ExecutionType::NonAtomic)
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_retry_logic(RetryLogic {
                            times: RetryTimes::Amount(5),
                            interval: Duration::Time(2),
                        })
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_error".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillError {
            error: "failed".to_string(),
        })
        .unwrap(),
    );
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1, message2],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Ticking 7 times (first acction successfull + first time second action + retry amount for second action) will send the callback with the error to the authorization contract
    for _ in 0..7 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

        setup.app.increase_time(5);
    }

    // Check it has been removed from the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Confirm we got the callback
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    // In this case the the first action was successful so we will receive a partially executed result with the amount actions that were successfully executed
    assert!(matches!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::PartiallyExecuted(1, _)
    ));
}

#[test]
fn successful_non_atomic_and_atomic_batches_together() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create two authorizations, one atomic and one non-atomic, with 2 actions each where both of them will succeed
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless-atomic")
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissionless-non-atomic")
            .with_action_batch(
                ActionBatchBuilder::new()
                    .with_execution_type(ExecutionType::NonAtomic)
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .with_action(
                        ActionBuilder::new()
                            .with_contract_address(&test_service_contract)
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
    ];

    // Create them and send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-atomic".to_string(),
            messages: vec![message1.clone(), message1.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-non-atomic".to_string(),
            messages: vec![message1.clone(), message1.clone()],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Ticking the first time will make the atomic batch succeed
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Confirm that we have only 1 batch left in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);

    // Confirm we got callback for the atomic batch
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert_eq!(query_confirmed_callbacks[0].messages.len(), 2);
    assert_eq!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::Success
    );

    // For the non-atomic batch we need to tick 2 times to process it
    for _ in 0..2 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.accounts[0],
        )
        .unwrap();

        setup.app.increase_time(5);
    }

    // Check that we have no more batches in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Confirm we got callback for the non-atomic batch
    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 2);
    for confirmed_callback in query_confirmed_callbacks.iter() {
        assert_eq!(confirmed_callback.messages.len(), 2);
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::Success
        );
    }
}

#[test]
fn reject_and_confirm_non_atomic_action_with_callback() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract = store_and_instantiate_test_service(&wasm, &setup.accounts[0], None);

    // We'll create an authorization with 2 actions, where both will succeed but second one needs to confirmed with a callback
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_execution_type(ExecutionType::NonAtomic)
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_retry_logic(RetryLogic {
                            times: RetryTimes::Indefinitely,
                            interval: Duration::Time(2),
                        })
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .with_callback_confirmation(ActionCallback {
                            contract_address: Addr::unchecked(test_service_contract.to_string()),
                            callback_message: Binary::from("Confirmed".as_bytes()),
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    // Create the authorization and send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestServiceExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1.clone(), message1],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Ticking the first time will make the first action succeed and re-add the batch to the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Confirm that we have it in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);

    // Ticking a second time will put the action in a pending callback confirmation state, removing it from the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Confirm that we have no more batches in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Sending the wrong callback will re-add the batch to the queue to retry the action
    let callback = Binary::from("Wrong".as_bytes());

    wasm.execute::<TestServiceExecuteMsg>(
        &test_service_contract,
        &TestServiceExecuteMsg::SendCallback {
            to: processor_contract.to_string(),
            callback,
        },
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Confirm that we have it in the queue again
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 1);

    setup.app.increase_time(5);

    // Tick again to retry the message
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Check that it's not in the queue
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    // Send the right callback to confirm
    let callback = Binary::from("Confirmed".as_bytes());

    wasm.execute::<TestServiceExecuteMsg>(
        &test_service_contract,
        &TestServiceExecuteMsg::SendCallback {
            to: processor_contract.to_string(),
            callback,
        },
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // It shouldn't be in the queue now and we should have the confirmed callback
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert_eq!(query_confirmed_callbacks[0].messages.len(), 2);
    assert_eq!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::Success
    );
}

#[test]
fn migration() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
            vec![setup.external_domain.clone()],
        );
    let test_service_contract =
        store_and_instantiate_test_service(&wasm, &setup.accounts[0], Some(&processor_contract));

    // Store it again to get a new code id
    let wasm_byte_code =
        std::fs::read(format!("{}/valence_test_service.wasm", ARTIFACTS_DIR)).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, &setup.accounts[0])
        .unwrap()
        .data
        .code_id;

    // Create an authorization with 1 action to migrate
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_action_batch(
            ActionBatchBuilder::new()
                .with_action(
                    ActionBuilder::new()
                        .with_contract_address(&test_service_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmMigrateMsg,
                            message: Message {
                                name: "migrate".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    // Create the authorization and send the message
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&valence_test_service::msg::MigrateMsg::Migrate {
            new_condition: true,
        })
        .unwrap(),
    );
    let message = ProcessorMessage::CosmwasmMigrateMsg {
        code_id,
        msg: binary,
    };

    // Send the message
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message],
        }),
        &[],
        &setup.accounts[2],
    )
    .unwrap();

    // Ticking the first time will make the migration succeed
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.accounts[0],
    )
    .unwrap();

    // Check that it was removed from the queue and we got the callback
    let query_med_prio_queue = wasm
        .query::<ProcessorQueryMsg, Vec<MessageBatch>>(
            &processor_contract,
            &ProcessorQueryMsg::GetQueue {
                from: None,
                to: None,
                priority: Priority::Medium,
            },
        )
        .unwrap();

    assert_eq!(query_med_prio_queue.len(), 0);

    let query_confirmed_callbacks = wasm
        .query::<QueryMsg, Vec<CallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ConfirmedCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_confirmed_callbacks.len(), 1);
    assert_eq!(query_confirmed_callbacks[0].messages.len(), 1);
    assert_eq!(
        query_confirmed_callbacks[0].execution_result,
        ExecutionResult::Success
    );

    // Check that indeed it was migrated by querying the contract
    let query_condition = wasm
        .query::<TestServiceQueryMsg, bool>(
            &test_service_contract,
            &TestServiceQueryMsg::Condition {},
        )
        .unwrap();

    assert!(query_condition);
}
