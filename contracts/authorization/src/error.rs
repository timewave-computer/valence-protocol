use cw_ownable::OwnershipError;
use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("This address is not allowed to execute this action")]
    Unauthorized {},

    #[error("Authorization must have a label")]
    EmptyLabel {},

    #[error("Authorization with label {0} already exists")]
    LabelAlreadyExists(String),

    #[error("Authorization must have at least one action")]
    NoActions {},

    #[error("All actions in an authorization must be executed in the same domain")]
    DifferentActionDomains {},

    #[error("Permissionless authorizations can't have high priority")]
    PermissionlessAuthorizationWithHighPriority {},

    #[error("Atomic authorizations can't have callback confirmations")]
    AtomicAuthorizationWithCallbackConfirmation {},

    #[error("External domain already exists")]
    ExternalDomainAlreadyExists(String),

    #[error("Domain {0} is not registered")]
    DomainIsNotRegistered(String),

    #[error("Authorization with label {0} does not exist")]
    AuthorizationDoesNotExist(String),

    #[error("Permissionless authorizations don't have a token that can be minted")]
    CantMintForPermissionlessAuthorization {},
}
