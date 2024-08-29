use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::processor::ProcessorMessage;

#[cw_serde]
pub struct PendingCallback {
    // Addr that will send the callback
    pub address: Addr,
    // Messages that are expected to be executed
    pub messages: Vec<ProcessorMessage>,
}
