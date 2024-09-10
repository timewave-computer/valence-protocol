use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use valence_authorization_utils::callback::ExecutionResult;

use crate::processor::MessageBatch;

#[cw_serde]
pub struct PendingCallback {
    // Address that needed to send the callback
    pub address: Addr,
    // Message that we are expecting
    pub callback_msg: Binary,
    // Batch that the callback is for (so that we can requeue if wrong callback is received)
    pub message_batch: MessageBatch,
}

#[cw_serde]
pub struct PendingPolytoneCallbackInfo {
    pub execution_result: ExecutionResult,
    pub state: PolytoneCallbackState,
}

#[cw_serde]
pub enum PolytoneCallbackState {
    Pending,
    TimedOut,
}
