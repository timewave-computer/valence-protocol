use thiserror::Error;

use crate::{domain::ConnectorError, service::ServiceError};

pub type ManagerResult<T> = Result<T, ManagerError>;

#[derive(Error, Debug)]
pub enum ManagerError {
    #[error("Generic Error: {0}")]
    Generic(String),

    #[error("Connector Error")]
    ConnectorError(#[from] ConnectorError),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),

    #[error("No instantiate data for account id: {0} | link id: {1}")]
    FailedToRetrieveAccountInitData(u64, u64),
}

impl ManagerError {
    pub fn generic_err(msg: impl Into<String>) -> Self {
        ManagerError::Generic(msg.into())
    }
}
