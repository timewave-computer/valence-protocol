use std::collections::BTreeSet;

use thiserror::Error;
use valence_library_utils::Id;

use crate::{config::ConfigError, domain::ConnectorError, library::LibraryError};

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
    LibraryError(#[from] LibraryError),

    #[error("Config authorization data is not empty")]
    AuthorizationDataNotDefault,

    #[error("Config has an id")]
    IdNotZero,

    #[error("Config has no owner")]
    OwnerEmpty,

    #[error("Config has no authorizations")]
    NoAuthorizations,

    #[error("Account id: {0} is not linked to any library")]
    AccountIdNotFoundInLinks(Id),

    #[error("Account id: {0} is not found in any library config")]
    AccountIdNotFoundInLibraries(Id),

    #[error("Library id: {0} is not linked to any library")]
    LibraryIdNotFoundInLinks(Id),

    #[error("Account ids: {:#?} is linked but not found in list", {0})]
    AccountIdNotFoundLink(BTreeSet<Id>),

    #[error("Account ids: {:#?} is found in config but not found in list", {0})]
    AccountIdNotFoundLibraryConfig(BTreeSet<Id>),

    #[error("Library ids: {:#?} is linked but not found in list", {0})]
    LibraryIdNotFoundLink(BTreeSet<Id>),

    #[error("No instantiate data for account id: {0} | link id: {1}")]
    FailedToRetrieveAccountInitData(u64, u64),

    #[error("Trying to instantiate a new program with an existing id: {0}")]
    ProgramIdAlreadyExists(u64),

    #[error("Failed to get processor address for this domain: {0}")]
    ProcessorAddrNotFound(String),

    #[error("Id must not be zero")]
    InvalidProgramId,

    #[error("Account was removed: {0}")]
    AccountWasRemoved(String),

    #[error("Library id is missing from the list of libraries: {0}")]
    LibraryIdIsMissing(u64),

    #[error("Authorization label was not found: {0}")]
    AuthorizationLabelNotFound(String),

    #[error("Authorization label already exists: {0}")]
    AuthorizationLabelExists(String),

    #[error("Funds transfer amount is zero - from: {0}, to: {1}")]
    FundsTransferAmountZero(String, String),

    #[error("Account address was not found in old program: {0}")]
    AccountNotFoundInOldProgram(String),

    #[error("Account id was not found in new program config: {0}")]
    AccountIdWasNotFound(u64),
}

impl ManagerError {
    pub fn generic_err(msg: impl Into<String>) -> Self {
        ManagerError::Generic(msg.into())
    }
}
