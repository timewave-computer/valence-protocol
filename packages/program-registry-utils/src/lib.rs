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
    /// "Lock" an id for a program to avoid race conditions
    ReserveId {},
    /// Save a new program config for the id
    SaveProgram { id: u64, program_config: Binary },
    /// Update a program config for the id
    UpdateProgram { id: u64, program_config: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets the most up to date program config for the id
    #[returns(ProgramResponse)]
    GetConfig { id: u64 },
    /// Gets the previous program config for the id
    /// returns None if there is no backup
    #[returns(Option<ProgramResponse>)]
    GetConfigBackup { id: u64 },
}

#[cw_serde]
pub struct ProgramResponse {
    pub id: u64,
    pub program_config: Binary,
}
