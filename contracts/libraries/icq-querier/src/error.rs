use cosmwasm_std::StdError;
use thiserror::Error;
use valence_library_utils::error::LibraryError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),
}

impl From<ContractError> for StdError {
    fn from(val: ContractError) -> Self {
        match val {
            ContractError::Std(std_error) => std_error,
            e => StdError::generic_err(e.to_string()),
        }
    }
}

impl From<ContractError> for LibraryError {
    fn from(val: ContractError) -> Self {
        LibraryError::Std(val.into())
    }
}
