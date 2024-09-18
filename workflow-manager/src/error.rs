use thiserror::Error;

use crate::{config::ConfigError, domain::ConnectorError, service::ServiceError};

pub type ManagerResult<T> = Result<T, ManagerError>;

#[derive(Error, Debug)]
pub enum ManagerError {
    #[error(transparent)]
    Error(#[from] anyhow::Error),
    
    #[error("Generic Error: {0}")]
    Generic(String),

    #[error("Connector Error")]
    ConnectorError(#[from] ConnectorError),

    #[error("Config Error")]
    ConfigError(#[from] ConfigError),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),

    #[error("Config authorization data is not empty")]
    AuthorizationDataNotDefault,

    #[error("No instantiate data for account id: {0} | link id: {1}")]
    FailedToRetrieveAccountInitData(u64, u64),
}

impl ManagerError {
    pub fn generic_err(msg: impl Into<String>) -> Self {
        ManagerError::Generic(msg.into())
    }
}
