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

    #[error(
        "Invalid queue position, queue position must be between 0 and the length of the queue"
    )]
    InvalidQueuePosition {},
}
