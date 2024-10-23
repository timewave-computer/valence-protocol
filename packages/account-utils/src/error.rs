use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    OwnershipError(#[from] OwnershipError),

    #[error("Unauthorized: {0}")]
    Unauthorized(#[from] UnauthorizedReason),
}

#[derive(Error, Debug, PartialEq)]
pub enum UnauthorizedReason {
    #[error("Unauthorized: Not the admin")]
    NotAdmin,

    #[error("Unauthorized: Not an approved service")]
    NotApprovedService,

    #[error("Unauthorized: Not the admin or an approved service")]
    NotAdminOrApprovedService,
}
