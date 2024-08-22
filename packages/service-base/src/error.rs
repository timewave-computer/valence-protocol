use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    OwnershipError(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized(#[from] UnauthorizedReason),
}

#[derive(Error, Debug, PartialEq)]
pub enum UnauthorizedReason {
    #[error("This address is not allowed to execute this action")]
    NotAllowed {},
}
