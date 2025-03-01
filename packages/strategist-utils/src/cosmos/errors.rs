use std::num::TryFromIntError;

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
