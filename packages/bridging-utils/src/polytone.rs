use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, CosmosMsg, Empty, SubMsgResponse, Uint64};

#[cw_serde]
pub enum PolytoneExecuteMsg {
    Execute {
        msgs: Vec<CosmosMsg<Empty>>,
        callback: Option<CallbackRequest>,
        timeout_seconds: Uint64,
    },
}

#[cw_serde]
pub struct CallbackRequest {
    pub receiver: String,
    pub msg: Binary,
}

#[cw_serde]
pub struct CallbackMessage {
    /// Initaitor on the note chain.
    pub initiator: Addr,
    /// Message sent by the initaitor. This _must_ be base64 encoded
    /// or execution will fail.
    pub initiator_msg: Binary,
    /// Data from the host chain.
    pub result: Callback,
}

#[cw_serde]
pub enum Callback {
    Query(Result<Vec<Binary>, ErrorResponse>),
    Execute(Result<ExecutionResponse, String>),
    FatalError(String),
}

#[cw_serde]
pub struct ExecutionResponse {
    /// The address on the remote chain that executed the messages.
    pub executed_by: String,
    /// Index `i` corresponds to the result of executing the `i`th
    /// message.
    pub result: Vec<SubMsgResponse>,
}

#[cw_serde]
pub struct ErrorResponse {
    /// The index of the first message who's execution failed.
    pub message_index: Uint64,
    /// The error that occured executing the message.
    pub error: String,
}
