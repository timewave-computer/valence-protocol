use alloy_primitives::Bytes;
use alloy_sol_types::SolValue;
use cosmwasm_std::Binary;

use crate::solidity_types::{ProcessorMessage, ProcessorMessageType};

pub fn encode() -> Binary {
    let processor_message = ProcessorMessage {
        messageType: ProcessorMessageType::Resume,
        // No data is required for the Resume message
        message: Bytes::new(),
    };

    Binary::new(processor_message.abi_encode())
}
