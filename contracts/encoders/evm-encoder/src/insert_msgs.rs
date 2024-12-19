use alloy_sol_types::SolValue;
use cosmwasm_std::{Binary, StdError, StdResult};
use valence_authorization_utils::authorization::{Priority, Subroutine};
use valence_encoder_utils::msg::Message;

use crate::{
    encode_subroutine,
    solidity_types::{InsertMsgs, ProcessorMessage, ProcessorMessageType},
    EVMLibrary,
};

pub fn encode(
    execution_id: u64,
    queue_position: u64,
    priority: Priority,
    subroutine: Subroutine,
    messages: Vec<Message>,
) -> StdResult<Binary> {
    let message = InsertMsgs {
        executionId: execution_id,
        queuePosition: queue_position,
        priority: priority.into(),
        subroutine: encode_subroutine(subroutine)?,
        messages: messages
            .iter()
            .map(|m| {
                let encoded = EVMLibrary::encode_message(&m.library, &m.data)?;
                Ok(encoded.into())
            })
            .collect::<Result<Vec<_>, StdError>>()?,
    };

    let processor_message = ProcessorMessage {
        messageType: ProcessorMessageType::InsertMsgs,
        message: message.abi_encode().into(),
    };

    Ok(Binary::new(processor_message.abi_encode()))
}
