use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{AnyMsg, CosmosMsg, StdError, StdResult, Uint64};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String, // Initial owner of the contract
    pub approved_libraries: Vec<String>,
    pub remote_domain_information: RemoteDomainInfo, // Remote domain information required to register the ICA and send messages to it
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    ApproveLibrary { library: String }, // Add library to approved list (only admin)
    RemoveLibrary { library: String },  // Remove library from approved list (only admin)
    ExecuteMsg { msgs: Vec<CosmosMsg> }, // Execute a list of Cosmos messages, useful to retrieve funds that were sent here by the owner for example.
    ExecuteIcaMsg { msgs: Vec<AnyMsg> }, // Execute a protobuf message on the ICA
    RegisterIca {},                      // Register the ICA on the remote chain
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<String>)]
    ListApprovedLibraries {}, // Get list of approved libraries
    #[returns(IcaState)]
    IcaState {}, // Get the state of the ICA
    #[returns(RemoteDomainInfo)]
    RemoteDomainInfo {}, // Get the remote domain information
}

#[cw_serde]
pub struct RemoteDomainInfo {
    pub connection_id: String,
    pub ica_timeout_seconds: Uint64, // relative timeout in seconds after which the packet times out
}

impl RemoteDomainInfo {
    pub fn validate(&self) -> StdResult<()> {
        if self.connection_id.is_empty() {
            return Err(StdError::generic_err("connection_id cannot be empty"));
        }
        if self.ica_timeout_seconds.is_zero() {
            return Err(StdError::generic_err("ica_timeout_seconds cannot be zero"));
        }

        Ok(())
    }
}

#[cw_serde]
pub enum IcaState {
    NotCreated, // Not created yet
    Closed,     // Was created but closed, so creation should be retriggered
    InProgress, // Creation is in progress, waiting for confirmation
    Created(IcaInformation),
}

#[cw_serde]
pub struct IcaInformation {
    pub address: String,
    pub port_id: String,
    pub controller_connection_id: String,
}

impl std::fmt::Display for IcaState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IcaState::NotCreated => write!(f, "NotCreated"),
            IcaState::Closed => write!(f, "Closed"),
            IcaState::InProgress => write!(f, "InProgress"),
            IcaState::Created(_) => write!(f, "Created"),
        }
    }
}
