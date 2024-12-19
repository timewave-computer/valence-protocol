use alloy_sol_types::SolValue;
use cosmwasm_std::{Binary, StdResult};
use valence_authorization_utils::authorization::Priority;

use crate::solidity_types::{self, EvictMsgs, ProcessorMessage, ProcessorMessageType};

pub fn encode(queue_position: u64, priority: Priority) -> StdResult<Binary> {
    let message = EvictMsgs {
        queuePosition: queue_position,
        priority: match priority {
            Priority::Medium => solidity_types::Priority::Medium,
            Priority::High => solidity_types::Priority::High,
        },
    };

    let processor_message = ProcessorMessage {
        messageType: ProcessorMessageType::EvictMsgs,
        message: message.abi_encode().into(),
    };

    Ok(Binary::new(processor_message.abi_encode()))
}
