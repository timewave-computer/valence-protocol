use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, HexBinary, StdError};
use valence_authorization_utils::{
    authorization::{Authorization, Priority, Subroutine},
    authorization_message::MessageType,
    msg::ProcessorMessage,
};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    IsValidLibrary { library: String },
    #[returns(Binary)]
    Encode { message: ProcessorMessageToEncode },
    #[returns(Binary)]
    Decode { message: ProcessorMessageToDecode },
}

#[cw_serde]
pub enum ProcessorMessageToEncode {
    SendMsgs {
        execution_id: u64,
        priority: Priority,
        subroutine: Subroutine,
        messages: Vec<Message>,
    },
    InsertMsgs {
        execution_id: u64,
        queue_position: u64,
        priority: Priority,
        subroutine: Subroutine,
        messages: Vec<Message>,
    },
    EvictMsgs {
        queue_position: u64,
        priority: Priority,
    },
    Pause {},
    Resume {},
}

#[cw_serde]
pub enum ProcessorMessageToDecode {
    HyperlaneCallback { callback: HexBinary },
}

#[cw_serde]
pub struct Message {
    pub library: String,
    pub data: Binary,
}

/// Converts a list of ProcessorMessages into a list of Messages that can be sent to the encoder
/// The authorization is used to get the library for each message to be sent
/// Only encodable messages are accepted
/// Should never error because all validations have been done during authorization creation and message validation
pub fn convert_into_encoder_messages(
    messages: Vec<ProcessorMessage>,
    authorization: &Authorization,
) -> Result<Vec<Message>, StdError> {
    messages
        .into_iter()
        .enumerate()
        .map(|(index, msg)| match msg {
            ProcessorMessage::EVMCall { msg } => {
                let function = authorization
                    .subroutine
                    .get_function_by_index(index)
                    .ok_or_else(|| StdError::generic_err("Function index not found"))?;

                let MessageType::EVMCall(_, lib) = &function.message_details().message_type
                else {
                    return Err(StdError::generic_err("Invalid message type"));
                };

                Ok(Message {
                    library: lib.to_string(),
                    data: msg,
                })
            }
            ProcessorMessage::EVMRawCall { msg } => Ok(Message {
                library: "no_library".to_string(),
                data: msg,
            }),
            _ => Err(StdError::generic_err("Message type not supported")),
        })
        .collect()
}
