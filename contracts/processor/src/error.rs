use cw_ownable::OwnershipError;
use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error(transparent)]
    Unauthorized(#[from] UnauthorizedReason),

    #[error("Processor is currently paused")]
    ProcessorPaused {},

    #[error("There is currently nothing to process")]
    NoMessagesToProcess {},

    #[error(transparent)]
    CallbackError(#[from] CallbackErrorReason),
}

#[derive(Error, Debug, PartialEq)]
pub enum CallbackErrorReason {
    #[error("Pending callback not found")]
    PendingCallbackNotFound {},

    #[error("Invalid callback sender")]
    InvalidCallbackSender {},
}

#[derive(Error, Debug, PartialEq)]
pub enum UnauthorizedReason {
    #[error("Only authorization module can execute this action")]
    NotAuthorizationModule {},

    #[error("Atomic execution can only be triggered by the processor itself")]
    NotProcessor {},
}
