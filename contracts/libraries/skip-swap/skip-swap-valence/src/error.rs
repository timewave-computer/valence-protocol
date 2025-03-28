use cosmwasm_std::{Decimal, StdError};
use thiserror::Error;

/// Custom error type for the Valence Skip Swap library
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized strategist: {address}")]
    UnauthorizedStrategist { address: String },

    #[error("Invalid asset pair: {input_asset} to {output_asset}")]
    InvalidAssetPair {
        input_asset: String,
        output_asset: String,
    },

    #[error("Invalid venue: {venue}")]
    InvalidVenue { venue: String },

    #[error("Slippage exceeds maximum: {slippage} > {max_slippage}")]
    ExcessiveSlippage { slippage: Decimal, max_slippage: Decimal },

    #[error("Missing destination for token: {token}")]
    MissingDestination { token: String },

    #[error("Incomplete swap operation: missing required details")]
    IncompleteSwapOperation,

    #[error("Invalid output amount: {min_output_amount} > {expected_output}")]
    InvalidOutputAmount {
        min_output_amount: String,
        expected_output: String,
    },
    
    #[error("Invalid route received from strategist")]
    InvalidRoute {},

    #[error("Route execution timed out")]
    RouteTimeout {},
    
    #[error("Unauthorized: {msg}")]
    Unauthorized { msg: String },
} 