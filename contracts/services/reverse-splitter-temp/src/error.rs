use cosmwasm_std::StdError;
use cw_ownable::OwnershipError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    OwnershipError(#[from] OwnershipError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unauthorized, Not the processor")]
    NotProcessor,
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
