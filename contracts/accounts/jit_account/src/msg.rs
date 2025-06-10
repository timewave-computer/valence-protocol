// Purpose: Message types for JIT account contract
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, CosmosMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub controller: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Execute a message on behalf of the account
    Execute { 
        msgs: Vec<CosmosMsg>,
    },
    /// Approve a library to execute messages
    ApproveLibrary { 
        library: String,
    },
    /// Remove approval for a library
    RemoveLibrary { 
        library: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the controller address
    #[returns(Addr)]
    GetController {},
    /// Check if a library is approved
    #[returns(bool)]
    IsLibraryApproved { 
        library: String,
    },
}
