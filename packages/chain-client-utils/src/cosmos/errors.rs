use std::num::TryFromIntError;

use alloy::transports::http::reqwest;
use cosmos_sdk_proto::prost::EncodeError;
use cosmrs::ErrorReport;
use tonic::Status;

use crate::common::error::StrategistError;

impl From<Status> for StrategistError {
    fn from(value: Status) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<ErrorReport> for StrategistError {
    fn from(value: ErrorReport) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<TryFromIntError> for StrategistError {
    fn from(value: TryFromIntError) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<serde_json::error::Error> for StrategistError {
    fn from(value: serde_json::error::Error) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<bip32::Error> for StrategistError {
    fn from(value: bip32::Error) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<cosmrs::tendermint::Error> for StrategistError {
    fn from(value: cosmrs::tendermint::Error) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<EncodeError> for StrategistError {
    fn from(value: EncodeError) -> Self {
        StrategistError::ParseError(value.to_string())
    }
}

impl From<tonic::transport::Error> for StrategistError {
    fn from(value: tonic::transport::Error) -> Self {
        StrategistError::ClientError(value.to_string())
    }
}

impl From<reqwest::Error> for StrategistError {
    fn from(value: reqwest::Error) -> Self {
        StrategistError::ClientError(value.to_string())
    }
}
