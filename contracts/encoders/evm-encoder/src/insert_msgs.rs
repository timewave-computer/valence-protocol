use alloy_sol_types::SolValue;
use cosmwasm_std::{Binary, StdError, StdResult};
use valence_authorization_utils::authorization::{Priority, Subroutine};
use valence_encoder_utils::msg::Message;

use crate::{
    encode_subroutine,
    solidity_types::{self, InsertMsgs, ProcessorMessage, ProcessorMessageType},
    EVMLibraryFunction,
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
        priority: match priority {
            Priority::Medium => solidity_types::Priority::Medium,
            Priority::High => solidity_types::Priority::High,
        },
        subroutine: encode_subroutine(subroutine)?,
        messages: messages
            .iter()
            .map(|m| {
                let encoded = EVMLibraryFunction::encode_message(&m.library, &m.data)?;
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
