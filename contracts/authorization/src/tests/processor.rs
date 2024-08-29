use cosmwasm_std::Binary;
use neutron_test_tube::{Module, Wasm};
use valence_authorization_utils::{
    authorization::{AuthorizationMode, PermissionType, Priority},
    domain::Domain,
};
use valence_processor_utils::processor::{MessageBatch, ProcessorMessage};

use crate::{
    error::{AuthorizationErrorReason, ContractError},
    msg::{ExecuteMsg, PermissionedMsg, PermissionlessMsg},
};
use valence_processor::msg::QueryMsg as ProcessorQueryMsg;

use super::{
    builders::{
        ActionBatchBuilder, ActionBuilder, AuthorizationBuilder, JsonBuilder, NeutronTestAppBuilder,
    },
    helpers::store_and_instantiate_authorization_with_processor_contract,
};

#[test]
fn user_enqueing_messages() {
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

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
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

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
        &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
    let setup = NeutronTestAppBuilder::new()
        .with_num_accounts(6)
        .build()
        .unwrap();

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
        &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
        &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
        &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
            &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
            &ExecuteMsg::PermissionedAction(PermissionedMsg::RemoveMsgs {
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

    // Trying to remove again will return an error because the queue is empty
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionedAction(PermissionedMsg::RemoveMsgs {
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
            &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
        &ExecuteMsg::PermissionedAction(PermissionedMsg::RemoveMsgs {
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
        &ExecuteMsg::PermissionedAction(PermissionedMsg::AddMsgs {
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
            &ExecuteMsg::PermissionedAction(PermissionedMsg::RemoveMsgs {
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
