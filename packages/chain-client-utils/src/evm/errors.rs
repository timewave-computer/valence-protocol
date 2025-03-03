use std::{num::ParseIntError, string::ParseError};

use alloy::{
    contract::Error,
    hex::FromHexError,
    primitives::AddressError,
    providers::PendingTransactionError,
    transports::{RpcError, TransportErrorKind},
};
use alloy_signer_local::LocalSignerError;

use crate::common::error::StrategistError;

impl From<LocalSignerError> for StrategistError {
    fn from(value: LocalSignerError) -> Self {
        StrategistError::ClientError(value.to_string())
    }
}

impl From<alloy::contract::Error> for StrategistError {
    fn from(value: Error) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<PendingTransactionError> for StrategistError {
    fn from(value: PendingTransactionError) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<TransportErrorKind> for StrategistError {
    fn from(value: TransportErrorKind) -> Self {
        StrategistError::TransactionError(value.to_string())
    }
}

impl From<ParseIntError> for StrategistError {
    fn from(value: ParseIntError) -> Self {
        StrategistError::TransactionError(value.to_string())
    }
}

impl From<AddressError> for StrategistError {
    fn from(value: AddressError) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<FromHexError> for StrategistError {
    fn from(value: FromHexError) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<ParseError> for StrategistError {
    fn from(value: ParseError) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<RpcError<TransportErrorKind>> for StrategistError {
    fn from(value: RpcError<TransportErrorKind>) -> Self {
        StrategistError::QueryError(value.to_string())
    }
}
