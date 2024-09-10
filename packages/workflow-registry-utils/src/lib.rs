use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw_ownable::cw_ownable_execute;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String, // Only admin can operate on the registry (for now)
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// "Lock" an id for a workflow to avoid race conditions
    ReserveId {},
    /// Save the
    SaveWorkflow {
        id: u64,
        workflow_config: Binary,
    },
    UpdateWorkflow {
        id: u64,
        workflow_config: Binary,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(WorkflowResponse)]
    GetConfig { id: u64 },
}

#[cw_serde]
pub struct WorkflowResponse {
    pub id: u64,
    pub workflow_config: Binary,
}
