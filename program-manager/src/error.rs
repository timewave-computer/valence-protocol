use std::collections::BTreeSet;

use thiserror::Error;
use valence_service_utils::Id;

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

    #[error("Config has an id")]
    IdNotZero,

    #[error("Config has no owner")]
    OwnerEmpty,

    #[error("Config has no authorizations")]
    NoAuthorizations,

    #[error("Account id: {0} is not linked to any service")]
    AccountIdNotFoundInLinks(Id),

    #[error("Account id: {0} is not found in any service config")]
    AccountIdNotFoundInServices(Id),

    #[error("Service id: {0} is not linked to any service")]
    ServiceIdNotFoundInLinks(Id),

    #[error("Account ids: {:#?} is linked but not found in list", {0})]
    AccountIdNotFoundLink(BTreeSet<Id>),

    #[error("Account ids: {:#?} is found in config but not found in list", {0})]
    AccountIdNotFoundServiceConfig(BTreeSet<Id>),

    #[error("Service ids: {:#?} is linked but not found in list", {0})]
    ServiceIdNotFoundLink(BTreeSet<Id>),

    #[error("No instantiate data for account id: {0} | link id: {1}")]
    FailedToRetrieveAccountInitData(u64, u64),

    #[error("Trying to instantiate a new program with an existing id: {0}")]
    ProgramIdAlreadyExists(u64),

    #[error("Failed to get processor address for this domain: {0}")]
    ProcessorAddrNotFound(String),
}

impl ManagerError {
    pub fn generic_err(msg: impl Into<String>) -> Self {
        ManagerError::Generic(msg.into())
    }
}
