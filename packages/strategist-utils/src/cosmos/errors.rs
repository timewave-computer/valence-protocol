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
