use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use cosmwasm_std::{
    testing::{message_info, mock_dependencies, mock_env},
    Addr, Binary, Empty,
};
use cw_utils::Duration;
use valence_authorization_utils::{
    authorization::{AtomicSubroutine, NonAtomicSubroutine, Priority, Subroutine},
    authorization_message::{Message, MessageDetails, MessageType},
    domain::Domain,
    function::{AtomicFunction, NonAtomicFunction, RetryLogic, RetryTimes},
};
use valence_encoder_utils::msg::{Message as EncoderMessage, ProcessorMessageToEncode, QueryMsg};

use crate::{
    contract::{instantiate, query},
    solidity_types::{EvictMsgs, InsertMsgs, ProcessorMessage, ProcessorMessageType, SendMsgs},
    EVMLibraryFunction,
};

#[test]
fn test_valid_combinations() {
    assert!(EVMLibraryFunction::is_valid("forwarder"));
    assert!(!EVMLibraryFunction::is_valid("invalid"));
    // PascalCase variants should not work as strings
    assert!(!EVMLibraryFunction::is_valid("Forwarder"));
}

#[test]
fn test_pause_message() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("any"), &[]);

    // Instantiate the contract
    instantiate(deps.as_mut(), env.clone(), info.clone(), Empty {}).unwrap();

    // Create a Pause message
    let pause_msg = ProcessorMessageToEncode::Pause {};

    // Encode using our contract
    let encoded = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Encode { message: pause_msg },
    )
    .unwrap();

    // Decode using Alloy
    let processor_msg = ProcessorMessage::abi_decode(&encoded, true).unwrap();

    // Verify the message type is Pause
    matches!(processor_msg.messageType, ProcessorMessageType::Pause);

    // Verify the message payload is empty for Pause
    assert_eq!(processor_msg.message.len(), 0);
}

#[test]
fn test_send_msgs() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("any"), &[]);

    instantiate(deps.as_mut(), env.clone(), info.clone(), Empty {}).unwrap();

    let message = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_forwarder_library::msg::FunctionMsgs::Forward {},
            ),
        )
        .unwrap(),
    );

    // Create a SendMsgs message with some test data
    let messages = vec![
        EncoderMessage {
            library: "forwarder".to_string(),
            data: message.clone(),
        },
        EncoderMessage {
            library: "forwarder".to_string(),
            data: message,
        },
    ];

    // This will be validated and available in the authorization contract
    let atomic_function = AtomicFunction {
        contract_address: valence_library_utils::LibraryAccountType::Addr(
            Address::from([1u8; 20]).to_string(),
        ),
        domain: Domain::External("Ethereum".to_string()),
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg, // This will be validated in the authorization contract so it can be anything here
            message: Message {
                name: "forwarder".to_string(),
                params_restrictions: None,
            },
        },
    };

    let retry_logic = Some(RetryLogic {
        times: RetryTimes::Amount(3),
        interval: Duration::Height(100),
    });

    let atomic_subroutine = AtomicSubroutine {
        functions: vec![atomic_function],
        retry_logic,
    };

    let subroutine = Subroutine::Atomic(atomic_subroutine);

    let send_msgs = ProcessorMessageToEncode::SendMsgs {
        execution_id: 1,
        priority: Priority::Medium,
        subroutine,
        messages,
    };

    // Encode using our contract
    let encoded = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Encode { message: send_msgs },
    )
    .unwrap();

    // Decode using Alloy
    let processor_msg = ProcessorMessage::abi_decode(&encoded, true).unwrap();

    // Verify message type
    matches!(processor_msg.messageType, ProcessorMessageType::SendMsgs);

    // Decode the SendMsgs struct from the message payload
    let decoded_send_msgs = SendMsgs::abi_decode(&processor_msg.message, true).unwrap();

    // Verify the decoded fields
    assert_eq!(decoded_send_msgs.executionId, 1);
    matches!(
        decoded_send_msgs.priority,
        crate::solidity_types::Priority::Medium
    );
    assert_eq!(decoded_send_msgs.messages.len(), 2);

    let subroutine = crate::solidity_types::AtomicSubroutine::abi_decode(
        &decoded_send_msgs.subroutine.subroutine,
        true,
    )
    .unwrap();

    assert_eq!(subroutine.functions.len(), 1);
    matches!(
        subroutine.retryLogic,
        crate::solidity_types::RetryLogic {
            times: crate::solidity_types::RetryTimes {
                retryType: crate::solidity_types::RetryTimesType::Amount,
                amount: 3
            },
            interval: crate::solidity_types::Duration {
                durationType: crate::solidity_types::DurationType::Height,
                value: 100
            }
        }
    );
}

#[test]
fn test_insert_msgs() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("any"), &[]);

    instantiate(deps.as_mut(), env.clone(), info.clone(), Empty {}).unwrap();

    // Create the message payload similar to send_msgs test
    let message = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_forwarder_library::msg::FunctionMsgs::Forward {},
            ),
        )
        .unwrap(),
    );

    let messages = vec![EncoderMessage {
        library: "forwarder".to_string(),
        data: message.clone(),
    }];

    // Create a non-atomic subroutine for variety
    let non_atomic_function = NonAtomicFunction {
        contract_address: valence_library_utils::LibraryAccountType::Addr(
            Address::from([1u8; 20]).to_string(),
        ),
        domain: Domain::External("Ethereum".to_string()),
        message_details: MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "forwarder".to_string(),
                params_restrictions: None,
            },
        },
        retry_logic: Some(RetryLogic {
            times: RetryTimes::Amount(3),
            interval: Duration::Height(100),
        }),
        callback_confirmation: None,
    };

    let non_atomic_subroutine = NonAtomicSubroutine {
        functions: vec![non_atomic_function],
    };

    let subroutine = Subroutine::NonAtomic(non_atomic_subroutine);

    // Create InsertMsgs message with a specific queue position
    let insert_msgs = ProcessorMessageToEncode::InsertMsgs {
        execution_id: 2,
        queue_position: 5, // Insert at position 5
        priority: Priority::High,
        subroutine,
        messages,
    };

    // Encode using our contract
    let encoded = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Encode {
            message: insert_msgs,
        },
    )
    .unwrap();

    // Decode using Alloy
    let processor_msg = ProcessorMessage::abi_decode(&encoded, true).unwrap();

    // Verify message type
    matches!(processor_msg.messageType, ProcessorMessageType::InsertMsgs);

    // Decode the InsertMsgs struct from the message payload
    let decoded_insert_msgs = InsertMsgs::abi_decode(&processor_msg.message, true).unwrap();

    // Verify the decoded fields
    assert_eq!(decoded_insert_msgs.executionId, 2);
    assert_eq!(decoded_insert_msgs.queuePosition, 5);
    matches!(
        decoded_insert_msgs.priority,
        crate::solidity_types::Priority::High
    );
    assert_eq!(decoded_insert_msgs.messages.len(), 1);
}

#[test]
fn test_evict_msgs() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("any"), &[]);

    instantiate(deps.as_mut(), env.clone(), info.clone(), Empty {}).unwrap();

    // Create EvictMsgs message
    let evict_msgs = ProcessorMessageToEncode::EvictMsgs {
        queue_position: 3,
        priority: Priority::High,
    };

    // Encode using our contract
    let encoded = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Encode {
            message: evict_msgs,
        },
    )
    .unwrap();

    // Decode using Alloy
    let processor_msg = ProcessorMessage::abi_decode(&encoded, true).unwrap();

    // Verify message type
    matches!(processor_msg.messageType, ProcessorMessageType::EvictMsgs);

    // Decode the EvictMsgs struct from the message payload
    let decoded_evict_msgs = EvictMsgs::abi_decode(&processor_msg.message, true).unwrap();

    // Verify the decoded fields
    assert_eq!(decoded_evict_msgs.queuePosition, 3);
    matches!(
        decoded_evict_msgs.priority,
        crate::solidity_types::Priority::High
    );
}

#[test]
fn test_resume() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("any"), &[]);

    instantiate(deps.as_mut(), env.clone(), info.clone(), Empty {}).unwrap();

    // Create Resume message
    let resume_msg = ProcessorMessageToEncode::Resume {};

    // Encode using our contract
    let encoded = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::Encode {
            message: resume_msg,
        },
    )
    .unwrap();

    // Decode using Alloy
    let processor_msg = ProcessorMessage::abi_decode(&encoded, true).unwrap();

    // Verify message type
    matches!(processor_msg.messageType, ProcessorMessageType::Resume);

    // For Resume messages, verify the message payload is empty
    assert_eq!(processor_msg.message.len(), 0);
}

// Helper test to verify different retry logic configurations
#[test]
fn test_send_msgs_with_different_retry_logic() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = message_info(&Addr::unchecked("any"), &[]);

    instantiate(deps.as_mut(), env.clone(), info.clone(), Empty {}).unwrap();

    // Create a basic message similar to previous tests
    let message = Binary::from(
        serde_json::to_vec(
            &valence_library_utils::msg::ExecuteMsg::<_, ()>::ProcessFunction(
                valence_forwarder_library::msg::FunctionMsgs::Forward {},
            ),
        )
        .unwrap(),
    );

    let messages = vec![EncoderMessage {
        library: "forwarder".to_string(),
        data: message,
    }];

    // Test different retry configurations
    let retry_configs = vec![
        RetryLogic {
            times: RetryTimes::Indefinitely,
            interval: Duration::Time(60),
        },
        RetryLogic {
            times: RetryTimes::Amount(65),
            interval: Duration::Height(10),
        },
    ];

    for retry_logic in retry_configs {
        let atomic_function = AtomicFunction {
            contract_address: valence_library_utils::LibraryAccountType::Addr(
                Address::from([1u8; 20]).to_string(),
            ),
            domain: Domain::External("Ethereum".to_string()),
            message_details: MessageDetails {
                message_type: MessageType::CosmwasmExecuteMsg,
                message: Message {
                    name: "forwarder".to_string(),
                    params_restrictions: None,
                },
            },
        };

        let atomic_subroutine = AtomicSubroutine {
            functions: vec![atomic_function],
            retry_logic: Some(retry_logic),
        };

        let send_msgs = ProcessorMessageToEncode::SendMsgs {
            execution_id: 1,
            priority: Priority::Medium,
            subroutine: Subroutine::Atomic(atomic_subroutine),
            messages: messages.clone(),
        };

        let encoded = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Encode { message: send_msgs },
        )
        .unwrap();

        let processor_msg = ProcessorMessage::abi_decode(&encoded, true).unwrap();
        matches!(processor_msg.messageType, ProcessorMessageType::SendMsgs);

        let decoded_send_msgs = SendMsgs::abi_decode(&processor_msg.message, true).unwrap();
        assert_eq!(decoded_send_msgs.executionId, 1);
        matches!(
            decoded_send_msgs.priority,
            crate::solidity_types::Priority::Medium
        );
    }
}
