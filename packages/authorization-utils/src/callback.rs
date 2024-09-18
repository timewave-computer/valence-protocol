use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_utils::Expiration;

use crate::{domain::Domain, msg::ProcessorMessage};

#[cw_serde]
pub struct ProcessorCallbackInfo {
    // Execution ID that the callback was for
    pub execution_id: u64,
    // Address that can send a bridge timeout or success for the message (if applied)
    pub bridge_callback_address: Option<Addr>,
    // Address that will send the callback for the processor
    pub processor_callback_address: Addr,
    // Domain that the callback came from
    pub domain: Domain,
    // Label of the authorization
    pub label: String,
    // Messages that were sent to the processor
    pub messages: Vec<ProcessorMessage>,
    // Optional ttl for re-sending in case of bridged timeouts
    pub ttl: Option<Expiration>,
    // Result of the execution
    pub execution_result: ExecutionResult,
}

#[cw_serde]
pub enum ExecutionResult {
    InProcess,
    // Everthing executed successfully
    Success,
    // Execution was rejected, and the reason
    Rejected(String),
    // Partially executed, for non-atomic action batches
    // Indicates how many actions were executed and the reason the next action was not executed
    PartiallyExecuted(usize, String),
    // Removed by Owner - happens when, from the authorization contract, a remove item from queue is sent
    RemovedByOwner,
    // Timeout - happens when the bridged message times out
    Timeout,
    // Unexpected error that should never happen but we'll store it here if it ever does
    UnexpectedError(String),
}

#[cw_serde]
pub enum PolytoneCallbackMsg {
    ExecutionID(u64),
    CreateProxy(String),
}
