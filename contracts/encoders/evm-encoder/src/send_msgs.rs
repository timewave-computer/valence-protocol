use alloy_sol_types::SolValue;
use cosmwasm_std::{Binary, StdError, StdResult};
use valence_authorization_utils::authorization::{Priority, Subroutine};
use valence_encoder_utils::msg::Message;

use crate::{
    encode_subroutine,
    solidity_types::{self, ProcessorMessage, ProcessorMessageType, SendMsgs},
    EVMLibraryFunction,
};

pub fn encode(
    execution_id: u64,
    priority: Priority,
    subroutine: Subroutine,
    messages: Vec<Message>,
) -> StdResult<Binary> {
    let message = SendMsgs {
        executionId: execution_id,
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
        messageType: ProcessorMessageType::SendMsgs,
        message: message.abi_encode().into(),
    };

    Ok(Binary::new(processor_message.abi_encode()))
}
