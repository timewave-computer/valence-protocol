use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Unauthorized: {0}")]
    Unauthorized(#[from] UnauthorizedReason),

    #[error("Authorization error: {0}")]
    Authorization(#[from] AuthorizationErrorReason),

    #[error("Message error: {0}")]
    Message(#[from] MessageErrorReason),

    #[error("External domain already exists")]
    ExternalDomainAlreadyExists(String),

    #[error("Domain {0} is not registered")]
    DomainIsNotRegistered(String),

    #[error("Invalid JSON passed: {error}")]
    InvalidJson { error: String },

    #[error("Execution ID {execution_id} does not exist")]
    ExecutionIDNotFound { execution_id: u64 },

    #[error("Unexpected current executions value, cannot be 0")]
    CurrentExecutionsIsZero {},
}

#[derive(Error, Debug, PartialEq)]
pub enum AuthorizationErrorReason {
    #[error("Authorization must have a label")]
    EmptyLabel {},

    #[error("Authorization with label {0} already exists")]
    LabelAlreadyExists(String),

    #[error("Authorization must have at least one function")]
    NoFunctions {},

    #[error("All functions in an authorization must be executed in the same domain")]
    DifferentFunctionDomains {},

    #[error("Permissionless authorizations can't have high priority")]
    PermissionlessWithHighPriority {},

    #[error("Authorization with label {0} does not exist")]
    DoesNotExist(String),

    #[error("Permissionless authorizations don't have a token that can be minted")]
    CantMintForPermissionless {},

    #[error("The authorization has reached its max concurrent executions")]
    MaxConcurrentExecutionsReached {},
}

#[derive(Error, Debug, PartialEq)]
pub enum UnauthorizedReason {
    #[error("This address is not allowed to execute this action")]
    NotAllowed {},

    #[error("The authorization is not enabled")]
    NotEnabled {},

    #[error("The authorization is expired")]
    Expired {},

    #[error("The authorization functions cant be executed yet")]
    NotActiveYet {},

    #[error("To proceed with this action, you must send exactly one token of this authorization")]
    RequiresOneToken {},

    #[error("The sender is not the authorized callback address")]
    UnauthorizedProcessorCallbackSender {},

    #[error("The polytone callback is not for a message initiated by the authorization contract")]
    InvalidPolytoneCallbackInitiator {},

    #[error("The polytone callback was not sent by an unauthorized address")]
    UnauthorizedPolytoneCallbackSender {},

    #[error("Creation of bridge was not timed out")]
    BridgeCreationNotTimedOut {},
}

#[derive(Error, Debug, PartialEq)]
pub enum MessageErrorReason {
    #[error("The amount of messages you send must match the amount of functions in the list")]
    InvalidAmount {},

    #[error("The message sent has a different type than expected")]
    InvalidType {},

    #[error("The message doesn't match the function")]
    DoesNotMatch {},

    #[error("The message doesn't pass all the parameter restrictions")]
    InvalidMessageParams {},

    #[error("The message can only have one top level key")]
    InvalidStructure {},

    #[error("Invalid polytone callback")]
    InvalidPolytoneCallback {},

    #[error("Messages are not retriable")]
    NotRetriable {},
}
