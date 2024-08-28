use thiserror::Error;

use crate::domain::cosmos_cw::CosmosCosmwasmError;

pub type ManagerResult<T> = Result<T, ManagerError>;

#[derive(Error, Debug)]
pub enum ManagerError {
    // #[error("Connector Error: {0}")]
    // Std(#[from] ConnectorError),
    #[error("Generic Error: {0}")]
    Generic(String),

    #[error(transparent)]
    CosmosCosmWasm(#[from] CosmosCosmwasmError),

    #[error("Chain not found for: {0}")]
    ChainInfoNotFound(String),

    #[error("Code ids not found for: {0}")]
    CodeIdsNotFound(String),

    #[error("No instantiate data for account id: {0} | link id: {1}")]
    FailedToRetrieveAccountInitData(u64, u64),
}

impl ManagerError {
    pub fn generic_err(msg: impl Into<String>) -> Self {
        ManagerError::Generic(msg.into())
    }
}
