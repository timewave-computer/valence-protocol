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

    #[error("Failed to create new client for: {0}")]
    FailedNewClient(String),

    #[error("Failed to create new wallet for: {0}")]
    FailedNewWalletInstance(String),
}

impl ManagerError {
    pub fn generic_err(msg: &str) -> Self {
        ManagerError::Generic(msg.to_string())
    }
}
