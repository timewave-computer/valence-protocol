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
    Unauthorized {},

    #[error("Unauthorized, Not the processor")]
    NotProcessor,
}
