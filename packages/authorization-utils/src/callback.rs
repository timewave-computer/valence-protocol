use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::{domain::Domain, msg::ProcessorMessage};

#[cw_serde]
pub struct CallbackInfo {
    // Execution ID that the callback was for
    pub execution_id: u64,
    // Addr that sent the callback
    pub address: Addr,
    // Domain that the callback came from
    pub domain: Domain,
    // Label of the authorization
    pub label: String,
    // Messages that were sent to the processor
    pub messages: Vec<ProcessorMessage>,
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
}
