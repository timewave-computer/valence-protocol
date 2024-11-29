use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unknown reply id: {0}")]
    UnknownReplyId(u64),

    #[error("Unsupported module: {0}")]
    UnsupportedModule(String),
}

impl From<ContractError> for StdError {
    fn from(val: ContractError) -> Self {
        match val {
            ContractError::Std(std_error) => std_error,
            e => StdError::generic_err(e.to_string()),
        }
    }
}
