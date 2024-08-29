use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::domain::Domain;

#[cw_serde]
pub struct PendingCallback {
    // Address that needs to send the callback
    pub address: Addr,
    // Domain that the callback comes from
    pub domain: Domain,
    // Label of the authorization
    pub label: String,
}

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
    // Result of the execution
    pub execution_result: ExecutionResult,
}

#[cw_serde]
pub enum ExecutionResult {
    // Everthing executed
    Executed,
    // Execution was rejected, and the reason
    Rejected(String),
    // Partially executed, for non-atomic action batches
    // Indicates how many actions were executed and the reason the next action was not executed
    PartiallyExecuted(u64, String),
}
