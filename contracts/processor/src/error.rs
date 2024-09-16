use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

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

    #[error("The polytone callback was not sent by polytone note address")]
    UnauthorizedPolytoneCallbackSender {},

    #[error("Invalid polytone callback received")]
    InvalidPolytoneCallback {},

    #[error("Polytone pending callback not found")]
    PolytonePendingCallbackNotFound {},

    #[error("Polytone callback still pending")]
    PolytoneCallbackStillPending {},

    #[error("Polytone callback status is not timed out, can only be retried if timed out")]
    PolytoneCallbackNotRetriable {},
}

#[derive(Error, Debug, PartialEq)]
pub enum UnauthorizedReason {
    #[error("Only authorization module can execute this action")]
    NotAuthorizationModule {},

    #[error("Atomic execution can only be triggered by the processor itself")]
    NotProcessor {},

    #[error("The polytone callback is not for a message initiated by the processor contract")]
    InvalidPolytoneCallbackInitiator {},

    #[error("This processor is on the main domain, it's not using polytone")]
    NotExternalDomainProcessor {},
}
