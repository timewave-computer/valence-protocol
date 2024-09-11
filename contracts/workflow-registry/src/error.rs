use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    OwnershipError(#[from] OwnershipError),

    #[error("Workflow already exists with id {0}")]
    WorkflowAlreadyExists(u64),
    #[error("Workflow doesn't exists with id {0}")]
    WorkflowDoesntExists(u64),
}
