use alloy_primitives::Bytes;
use alloy_sol_types::SolValue;
use cosmwasm_std::Binary;
use valence_encoder_utils::processor::solidity_types::{ProcessorMessage, ProcessorMessageType};

pub fn encode() -> Binary {
    let processor_message = ProcessorMessage {
        messageType: ProcessorMessageType::Pause,
        // No data is required for the Pause message
        message: Bytes::new(),
    };

    Binary::new(processor_message.abi_encode())
}
