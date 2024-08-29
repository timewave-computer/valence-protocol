use cw_ownable::OwnershipError;
use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error("Unauthorized, only authorization module can execute this action")]
    Unauthorized {},

    #[error("Processor is currently paused")]
    ProcessorPaused {},

    #[error("There is currently nothing to process")]
    NoMessagesToProcess {},

    #[error("Processing ID not found")]
    ProcessingIDNotFound {},
}
