use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    OwnershipError(#[from] OwnershipError),

    #[error("Program already exists with id {0}")]
    ProgramAlreadyExists(u64),
    #[error("Program doesn't exists with id {0}")]
    ProgramDoesntExists(u64),

    #[error("Unauthorized: This id is reserved to {0}")]
    UnauthorizedToSave(String),
    #[error("Unauthorized: This id can be only updated by {0}")]
    UnauthorizedToUpdate(String),
}
