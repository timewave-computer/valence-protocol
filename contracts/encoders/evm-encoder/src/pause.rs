use alloy_primitives::Bytes;
use alloy_sol_types::SolValue;
use cosmwasm_std::Binary;

use crate::solidity_types::{ProcessorMessage, ProcessorMessageType};

fn encode() -> Binary {
    let processor_message = ProcessorMessage {
        messageType: ProcessorMessageType::Pause,
        // No data is required for the Pause message
        message: Bytes::new(),
    };

    Binary::new(processor_message.abi_encode())
}
