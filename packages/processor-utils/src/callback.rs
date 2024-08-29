use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::processor::MessageBatch;

#[cw_serde]
pub struct PendingCallback {
    // Address that needed to send the callback
    pub address: Addr,
    pub message_batch: MessageBatch,
}
