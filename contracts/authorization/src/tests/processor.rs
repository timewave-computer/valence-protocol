use cosmwasm_std::{Addr, Binary, Coin, Uint128};
use cw_utils::Duration;
use margined_neutron_std::types::cosmos::bank::v1beta1::QueryBalanceRequest;
use neutron_test_tube::{Account, Bank, Module, Wasm};
use valence_authorization_utils::{
    authorization::{
        AtomicSubroutine, AuthorizationModeInfo, PermissionTypeInfo, Priority, Subroutine,
    },
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{
        AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder, JsonBuilder,
        NonAtomicFunctionBuilder, NonAtomicSubroutineBuilder,
    },
    callback::{ExecutionResult, ProcessorCallbackInfo},
    domain::Domain,
    function::{FunctionCallback, RetryLogic, RetryTimes},
    msg::{ExecuteMsg, PermissionedMsg, PermissionlessMsg, ProcessorMessage, QueryMsg},
};
use valence_library_utils::LibraryAccountType;
use valence_processor_utils::{msg::InternalProcessorMsg, processor::MessageBatch};

use crate::{
    contract::build_tokenfactory_denom,
    error::{AuthorizationErrorReason, ContractError},
    tests::helpers::{wait_for_height, ARTIFACTS_DIR},
};
use valence_processor_utils::msg::{
    ExecuteMsg as ProcessorExecuteMsg, PermissionlessMsg as ProcessorPermissionlessMsg,
    QueryMsg as ProcessorQueryMsg,
};

use valence_processor::error::{ContractError as ProcessorContractError, UnauthorizedReason};

use valence_test_library::msg::{
    ExecuteMsg as TestLibraryExecuteMsg, QueryMsg as TestLibraryQueryMsg,
};

use super::{
    builders::NeutronTestAppBuilder,
    helpers::{
        store_and_instantiate_authorization_with_processor_contract,
        store_and_instantiate_test_library,
    },
};

#[test]
fn user_enqueing_messages() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );

    // We'll create two authorization, one with high priority and one without to test the correct queueing of messages
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_max_concurrent_executions(10)
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(AtomicFunctionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_max_concurrent_executions(10)
            .with_mode(AuthorizationModeInfo::Permissioned(
                PermissionTypeInfo::WithoutCallLimit(vec![
                    setup.subowner_addr.to_string(),
                    setup.user_accounts[0].address().to_string(),
                ]),
            ))
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(AtomicFunctionBuilder::new().build())
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
        &setup.owner_accounts[0],
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
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
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
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
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
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );

    // We'll create an authorization with max concurrent executions and check that we can't queue more than that
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_max_concurrent_executions(3)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(AtomicFunctionBuilder::new().build())
                .build(),
        )
        .build()];

    // Let's create the authorization
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
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
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
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
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
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
        &setup.owner_accounts[0],
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
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );

    // We'll create two authorization, one with high priority and one without to test the correct queueing of messages
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_max_concurrent_executions(10)
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(AtomicFunctionBuilder::new().build())
                    .build(),
            )
            .build(),
        AuthorizationBuilder::new()
            .with_label("permissioned-without-limit")
            .with_max_concurrent_executions(10)
            .with_mode(AuthorizationModeInfo::Permissioned(
                PermissionTypeInfo::WithoutCallLimit(vec![
                    setup.subowner_addr.to_string(),
                    setup.user_accounts[0].address().to_string(),
                ]),
            ))
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(AtomicFunctionBuilder::new().build())
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
        &setup.owner_accounts[0],
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
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap();

        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![message.clone()],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
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
        &setup.owner_accounts[0],
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
        &setup.owner_accounts[0],
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
        &setup.owner_accounts[0],
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
            &setup.owner_accounts[0],
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
            &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // All of them, confirmed and not confirmed
    assert_eq!(query_callbacks.len(), 13);

    let expected_callbacks = [1, 3, 5, 7, 9];
    for (index, confirmed_callback) in query_callbacks.iter().enumerate() {
        if expected_callbacks.contains(&index) {
            assert_eq!(
                confirmed_callback.execution_result,
                ExecutionResult::RemovedByOwner
            );
        } else {
            assert_eq!(
                confirmed_callback.execution_result,
                ExecutionResult::InProcess
            );
        }
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
            &setup.owner_accounts[0],
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
            &setup.owner_accounts[0],
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
        &setup.owner_accounts[0],
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
        &setup.owner_accounts[0],
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
            &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    // We removed 6 more
    assert_eq!(query_callbacks.len(), 19);
    // We removed starting from the back
    let expected_callbacks = [1, 3, 5, 7, 9, 13, 14, 15, 16, 17, 18];
    for (index, confirmed_callback) in query_callbacks.iter().enumerate() {
        if expected_callbacks.contains(&index) {
            assert_eq!(
                confirmed_callback.execution_result,
                ExecutionResult::RemovedByOwner
            );
        } else {
            assert_eq!(
                confirmed_callback.execution_result,
                ExecutionResult::InProcess
            );
        }
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
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // Let's create an authorization for sending a message to the test library that doesn't even exist on the contract
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_max_concurrent_executions(10)
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(test_library_contract))
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Let's try to send an invalid message to the test library
    let binary =
        Binary::from(serde_json::to_vec(&JsonBuilder::new().main("method").build()).unwrap());
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send it, which will add it to the queue
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message.clone()],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
        &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert!(matches!(
        query_callbacks[0].execution_result,
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
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // Let's create two authorizations (one atomic and one non atomic) that will always fail and see that when they fail, they are put back on the back in the queue
    // and when the retrying cooldown is not reached, they are shifted to the back of the queue
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless-atomic")
            .with_max_concurrent_executions(2)
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::Addr(
                                test_library_contract.clone(),
                            ))
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
            .with_subroutine(
                NonAtomicSubroutineBuilder::new()
                    .with_function(
                        NonAtomicFunctionBuilder::new()
                            .with_contract_address(&test_library_contract.clone())
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_error".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "will_error".to_string(),
                                            "error".to_string(),
                                        ]),
                                    ]),
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Let's send 2 messages to the queue
    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillError {
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
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-non-atomic".to_string(),
            messages: vec![message.clone()],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Check there are no confirmed callbacks
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert!(query_callbacks
        .iter()
        .all(|callback| callback.execution_result == ExecutionResult::InProcess));

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
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Now the first message should be back at the beginning of the queue, and if we tick, we'll just shift the queue but not attempt to process anything
    // because retry method has not been reached
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
        )
        .unwrap();

    assert!(response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_function_back_to_queue")));

    // Let's tick again to check that the same happens with the non atomic function
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
        )
        .unwrap();

    assert!(response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_function_back_to_queue")));

    // Let's increase the block height enouch to trigger the retry and double check that the function is tried again
    let current_height = setup.app.get_block_height() as u64;
    wait_for_height(&setup.app, current_height + 50);

    // If we tick know we will try to execute
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
        )
        .unwrap();

    assert!(!response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_function_back_to_queue")));

    // Same for non atomic function
    let response = wasm
        .execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
        )
        .unwrap();

    assert!(!response.events.iter().any(|e| e.ty == "wasm"
        && e.attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "pushed_function_back_to_queue")));
}

#[test]
fn higher_priority_queue_is_processed_first() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create two authorizations, one with high priority and one without, and we'll enqueue two messages for both
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless")
            .with_max_concurrent_executions(10)
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::Addr(
                                test_library_contract.clone(),
                            ))
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
            .with_mode(AuthorizationModeInfo::Permissioned(
                PermissionTypeInfo::WithoutCallLimit(vec![setup.user_accounts[0]
                    .address()
                    .to_string()]),
            ))
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::Addr(
                                test_library_contract.clone(),
                            ))
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Let's execute two of each
    for _ in 0..2 {
        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissionless".to_string(),
                messages: vec![message.clone()],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap();

        wasm.execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-without-limit".to_string(),
                messages: vec![message.clone()],
                ttl: None,
            }),
            &[],
            &setup.user_accounts[0],
        )
        .unwrap();
    }

    // Let's check that the high priority queue is processed first
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert!(query_callbacks.iter().any(|callback| {
        callback.execution_id == 1 && callback.execution_result == ExecutionResult::Success
    }));

    // Now let's tick again to process the other message in the high priority queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert!(query_callbacks.iter().any(|callback| {
        callback.execution_id == 3 && callback.execution_result == ExecutionResult::Success
    }));

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
            &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    let expected_callbacks = [0, 1, 2, 3];
    for (index, confirmed_callback) in query_callbacks.iter().enumerate() {
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::Success
        );
        assert_eq!(confirmed_callback.execution_id, expected_callbacks[index]);
    }
}

#[test]
fn retry_multi_function_atomic_batch_until_success() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization with 3 functions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_retry_logic(RetryLogic {
                    times: RetryTimes::Indefinitely,
                    interval: Duration::Time(2),
                })
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            test_library_contract.clone(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            test_library_contract.clone(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed_if_true".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            test_library_contract.clone(),
                        ))
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary =
        Binary::from(serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceedIfTrue {}).unwrap());
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1.clone(), message2, message1],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
            &setup.owner_accounts[0],
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
    wasm.execute::<TestLibraryExecuteMsg>(
        &test_library_contract.clone(),
        &TestLibraryExecuteMsg::SetCondition { condition: true },
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Ticking now will make it succeed
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert_eq!(query_callbacks[0].messages.len(), 3);
    assert_eq!(
        query_callbacks[0].execution_result,
        ExecutionResult::Success
    );
}

#[test]
fn retry_multi_function_non_atomic_batch_until_success() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization with 3 functions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_subroutine(
            NonAtomicSubroutineBuilder::new()
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
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
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary =
        Binary::from(serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceedIfTrue {}).unwrap());
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1.clone(), message2, message1],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
        &setup.owner_accounts[0],
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
    for i in 0..5 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
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
        // Verify the current retry we are at
        assert_eq!(
            query_med_prio_queue[0].retry.clone().unwrap().retry_amounts,
            i + 1
        );
        setup.app.increase_time(5);
    }

    // Change the condition to true to make it succeed
    wasm.execute::<TestLibraryExecuteMsg>(
        &test_library_contract,
        &TestLibraryExecuteMsg::SetCondition { condition: true },
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Tick again will move now to the 3rd function but not process it, just re-add it to the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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
    // Verify that after moving to the next function, the current retries has been reset
    assert_eq!(query_med_prio_queue[0].retry, None);

    // Last tick will process the last message and send the callback
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert_eq!(query_callbacks[0].messages.len(), 3);
}

#[test]
fn failed_atomic_batch_after_retries() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization with 3 functions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_retry_logic(RetryLogic {
                    times: RetryTimes::Amount(5),
                    interval: Duration::Time(2),
                })
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            test_library_contract.clone(),
                        ))
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            test_library_contract.clone(),
                        ))
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillError {
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
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
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
                    subroutine: Subroutine::Atomic(AtomicSubroutine {
                        functions: vec![],
                        retry_logic: None,
                    }),
                    priority: Priority::Medium,
                    retry: None,
                    expiration_time: cw_utils::Expiration::AtTime(
                        setup.app.get_block_timestamp().plus_seconds(100),
                    ),
                },
            }),
            &[],
            &setup.owner_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ProcessorContractError::Unauthorized(UnauthorizedReason::NotProcessor {})
            .to_string()
            .as_str()
    ));

    // Ticking 6 times (first time + retry amount) will send the callback with the error to the authorization contract
    for _ in 0..6 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert!(matches!(
        query_callbacks[0].execution_result,
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
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization with 3 functions, where the first one and third will always succeed but the second one will fail until we modify the contract to succeed
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissioned")
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithCallLimit(vec![(
                setup.user_accounts[0].address(),
                // Mint one tokens to execute once
                Uint128::new(1),
            )]),
        ))
        .with_subroutine(
            NonAtomicSubroutineBuilder::new()
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };
    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillError {
            error: "failed".to_string(),
        })
        .unwrap(),
    );
    let message2 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    let permission_token = build_tokenfactory_denom(&authorization_contract, "permissioned");

    // Send the messages
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissioned".to_string(),
            messages: vec![message1, message2],
            ttl: None,
        }),
        &[Coin::new(Uint128::one(), permission_token.to_string())],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Ticking 7 times (first function successfull + first time second function + retry amount for second function) will send the callback with the error to the authorization contract
    for _ in 0..7 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    // In this case the the first function was successful so we will receive a partially executed result with the amount functions that were successfully executed
    assert!(matches!(
        query_callbacks[0].execution_result,
        ExecutionResult::PartiallyExecuted(1, _)
    ));

    // Verify that neither the contract nor the user has the token (it was burned)
    let bank = Bank::new(&setup.app);
    let balance = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_accounts[0].address(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(balance.balance.unwrap().amount, "0");

    let balance = bank
        .query_balance(&QueryBalanceRequest {
            address: authorization_contract.clone(),
            denom: permission_token,
        })
        .unwrap();

    assert_eq!(balance.balance.unwrap().amount, "0");
}

#[test]
fn successful_non_atomic_and_atomic_batches_together() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create two authorizations, one atomic and one non-atomic, with 2 functions each where both of them will succeed
    let authorizations = vec![
        AuthorizationBuilder::new()
            .with_label("permissionless-atomic")
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::Addr(
                                test_library_contract.clone(),
                            ))
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(LibraryAccountType::Addr(
                                test_library_contract.clone(),
                            ))
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
            .with_subroutine(
                NonAtomicSubroutineBuilder::new()
                    .with_function(
                        NonAtomicFunctionBuilder::new()
                            .with_contract_address(&test_library_contract.clone())
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "will_succeed".to_string(),
                                    params_restrictions: None,
                                },
                            })
                            .build(),
                    )
                    .with_function(
                        NonAtomicFunctionBuilder::new()
                            .with_contract_address(&test_library_contract)
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-atomic".to_string(),
            messages: vec![message1.clone(), message1.clone()],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless-non-atomic".to_string(),
            messages: vec![message1.clone(), message1.clone()],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Ticking the first time will make the atomic batch succeed
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 2);
    assert_eq!(query_callbacks[0].messages.len(), 2);
    assert_eq!(
        query_callbacks[0].execution_result,
        ExecutionResult::Success
    );

    // For the non-atomic batch we need to tick 2 times to process it
    for _ in 0..2 {
        wasm.execute::<ProcessorExecuteMsg>(
            &processor_contract,
            &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
            &[],
            &setup.owner_accounts[0],
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
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 2);
    for confirmed_callback in query_callbacks.iter() {
        assert_eq!(confirmed_callback.messages.len(), 2);
        assert_eq!(
            confirmed_callback.execution_result,
            ExecutionResult::Success
        );
    }
}

#[test]
fn reject_and_confirm_non_atomic_function_with_callback() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization with 2 functions, where both will succeed but second one needs to confirmed with a callback
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_subroutine(
            NonAtomicSubroutineBuilder::new()
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
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
                        .with_callback_confirmation(FunctionCallback {
                            contract_address: Addr::unchecked(test_library_contract.to_string()),
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );
    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissionless".to_string(),
            messages: vec![message1.clone(), message1],
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Ticking the first time will make the first function succeed and re-add the batch to the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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

    // Ticking a second time will put the function in a pending callback confirmation state, removing it from the queue
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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

    // Sending the wrong callback will re-add the batch to the queue to retry the function
    let callback = Binary::from("Wrong".as_bytes());

    wasm.execute::<TestLibraryExecuteMsg>(
        &test_library_contract,
        &TestLibraryExecuteMsg::SendCallback {
            to: processor_contract.to_string(),
            callback,
        },
        &[],
        &setup.owner_accounts[0],
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
        &setup.owner_accounts[0],
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

    wasm.execute::<TestLibraryExecuteMsg>(
        &test_library_contract,
        &TestLibraryExecuteMsg::SendCallback {
            to: processor_contract.to_string(),
            callback,
        },
        &[],
        &setup.owner_accounts[0],
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

    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert_eq!(query_callbacks[0].messages.len(), 2);
    assert_eq!(
        query_callbacks[0].execution_result,
        ExecutionResult::Success
    );
}

#[test]
fn refund_and_burn_tokens_after_callback() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);
    let bank = Bank::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization that we'll force to fail and succeed once to check that refund and burning works
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissioned-with-limit")
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithCallLimit(vec![(
                setup.user_accounts[0].address(),
                // Mint two tokens to also check concurrent executions
                Uint128::new(2),
            )]),
        ))
        .with_subroutine(
            NonAtomicSubroutineBuilder::new()
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
                        .with_message_details(MessageDetails {
                            message_type: MessageType::CosmwasmExecuteMsg,
                            message: Message {
                                name: "will_succeed_if_true".to_string(),
                                params_restrictions: None,
                            },
                        })
                        .build(),
                )
                .build(),
        )
        .build()];

    // Create the authorization and messages that will be sent
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary =
        Binary::from(serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceedIfTrue {}).unwrap());

    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // The token that was minted to the user
    let permission_token =
        build_tokenfactory_denom(&authorization_contract, "permissioned-with-limit");

    // Sending the message will enqueue into the processor
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissioned-with-limit".to_string(),
            messages: vec![message1.clone()],
            ttl: None,
        }),
        &[Coin::new(Uint128::one(), permission_token.to_string())],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Trying to send again will fail because there's only 1 concurrent execution allowed
    let error = wasm
        .execute::<ExecuteMsg>(
            &authorization_contract,
            &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
                label: "permissioned-with-limit".to_string(),
                messages: vec![message1.clone()],
                ttl: None,
            }),
            &[Coin::new(Uint128::one(), permission_token.to_string())],
            &setup.user_accounts[0],
        )
        .unwrap_err();

    assert!(error.to_string().contains(
        ContractError::Authorization(AuthorizationErrorReason::MaxConcurrentExecutionsReached {})
            .to_string()
            .as_str()
    ));

    // Check the balance of the user to verify that they don't have any tokens left
    let balance = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_accounts[0].address(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(balance.balance.unwrap().amount, "1");

    // Ticking the processor will make it fail and send a Rejected callback, which should refund the token to the user
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Check that the user has been refunded
    let balance = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_accounts[0].address(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(balance.balance.unwrap().amount, "2");

    // Now we should be able to enqueue again
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissioned-with-limit".to_string(),
            messages: vec![message1],
            ttl: None,
        }),
        &[Coin::new(Uint128::one(), permission_token.to_string())],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Modify the test library to make the message succeed when it eventually executes
    wasm.execute::<TestLibraryExecuteMsg>(
        &test_library_contract,
        &TestLibraryExecuteMsg::SetCondition { condition: true },
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Ticking the processor will make it succeed and send a Success callback, which should burn the token instead of refunding it
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // Verify that user still has 1 token and contract doesn't have any
    let user_balance = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_accounts[0].address(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(user_balance.balance.unwrap().amount, "1");

    let contract_balance = bank
        .query_balance(&QueryBalanceRequest {
            address: authorization_contract.clone(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(contract_balance.balance.unwrap().amount, "0");
}

#[test]
fn burn_tokens_after_removed_by_owner() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);
    let bank = Bank::new(&setup.app);

    let (authorization_contract, _) = store_and_instantiate_authorization_with_processor_contract(
        &setup.app,
        &setup.owner_accounts[0],
        setup.owner_addr.to_string(),
        vec![setup.subowner_addr.to_string()],
    );
    let test_library_contract =
        store_and_instantiate_test_library(&wasm, &setup.owner_accounts[0], None);

    // We'll create an authorization that we'll remove by the owner to check that the token is correctly burned
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissioned-with-limit")
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithCallLimit(vec![(
                setup.user_accounts[0].address(),
                // Mint one token to allow 1 execution
                Uint128::one(),
            )]),
        ))
        .with_subroutine(
            NonAtomicSubroutineBuilder::new()
                .with_function(
                    NonAtomicFunctionBuilder::new()
                        .with_contract_address(&test_library_contract)
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

    // Create the authorization and messages that will be sent
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::CreateAuthorizations { authorizations }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&TestLibraryExecuteMsg::WillSucceed { execution_id: None }).unwrap(),
    );

    let message1 = ProcessorMessage::CosmwasmExecuteMsg { msg: binary };

    // The token that was minted to the user
    let permission_token =
        build_tokenfactory_denom(&authorization_contract, "permissioned-with-limit");

    // Sending the message will enqueue into the processor
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionlessAction(PermissionlessMsg::SendMsgs {
            label: "permissioned-with-limit".to_string(),
            messages: vec![message1.clone()],
            ttl: None,
        }),
        &[Coin::new(Uint128::one(), permission_token.to_string())],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Let's balance of the user to verify that he doesn't have any tokens left
    let balance = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_accounts[0].address(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(balance.balance.unwrap().amount, "0");

    // Verify that the contract has the token in escrow
    let contract_balance = bank
        .query_balance(&QueryBalanceRequest {
            address: authorization_contract.clone(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(contract_balance.balance.unwrap().amount, "1");

    // Remove the message by owner
    wasm.execute::<ExecuteMsg>(
        &authorization_contract,
        &ExecuteMsg::PermissionedAction(PermissionedMsg::EvictMsgs {
            domain: Domain::Main,
            queue_position: 0,
            priority: Priority::Medium,
        }),
        &[],
        &setup.owner_accounts[0],
    )
    .unwrap();

    // We should have a confirmed callback  with RemovedByOwner result
    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert_eq!(
        query_callbacks[0].execution_result,
        ExecutionResult::RemovedByOwner
    );

    // Token should have been burnt, not refunded
    let contract_balance = bank
        .query_balance(&QueryBalanceRequest {
            address: authorization_contract.clone(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(contract_balance.balance.unwrap().amount, "0");

    // User should still have 0 tokens
    let balance = bank
        .query_balance(&QueryBalanceRequest {
            address: setup.user_accounts[0].address(),
            denom: permission_token.clone(),
        })
        .unwrap();

    assert_eq!(balance.balance.unwrap().amount, "0");
}

#[test]
fn migration() {
    let setup = NeutronTestAppBuilder::new().build().unwrap();

    let wasm = Wasm::new(&setup.app);

    let (authorization_contract, processor_contract) =
        store_and_instantiate_authorization_with_processor_contract(
            &setup.app,
            &setup.owner_accounts[0],
            setup.owner_addr.to_string(),
            vec![setup.subowner_addr.to_string()],
        );
    let test_library_contract = store_and_instantiate_test_library(
        &wasm,
        &setup.owner_accounts[0],
        Some(&processor_contract),
    );

    // Store it again to get a new code id
    let wasm_byte_code =
        std::fs::read(format!("{}/valence_test_library.wasm", ARTIFACTS_DIR)).unwrap();

    let code_id = wasm
        .store_code(&wasm_byte_code, None, &setup.owner_accounts[0])
        .unwrap()
        .data
        .code_id;

    // Create an authorization with 1 function to migrate
    let authorizations = vec![AuthorizationBuilder::new()
        .with_label("permissionless")
        .with_subroutine(
            AtomicSubroutineBuilder::new()
                .with_function(
                    AtomicFunctionBuilder::new()
                        .with_contract_address(LibraryAccountType::Addr(
                            test_library_contract.clone(),
                        ))
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
        &setup.owner_accounts[0],
    )
    .unwrap();

    let binary = Binary::from(
        serde_json::to_vec(&valence_test_library::msg::MigrateMsg::Migrate {
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
            ttl: None,
        }),
        &[],
        &setup.user_accounts[0],
    )
    .unwrap();

    // Ticking the first time will make the migration succeed
    wasm.execute::<ProcessorExecuteMsg>(
        &processor_contract,
        &ProcessorExecuteMsg::PermissionlessAction(ProcessorPermissionlessMsg::Tick {}),
        &[],
        &setup.owner_accounts[0],
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

    let query_callbacks = wasm
        .query::<QueryMsg, Vec<ProcessorCallbackInfo>>(
            &authorization_contract,
            &QueryMsg::ProcessorCallbacks {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(query_callbacks.len(), 1);
    assert_eq!(query_callbacks[0].messages.len(), 1);
    assert_eq!(
        query_callbacks[0].execution_result,
        ExecutionResult::Success
    );

    // Check that indeed it was migrated by querying the contract
    let query_condition = wasm
        .query::<TestLibraryQueryMsg, bool>(
            &test_library_contract,
            &TestLibraryQueryMsg::Condition {},
        )
        .unwrap();

    assert!(query_condition);
}
