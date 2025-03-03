use base_client::NANOS_IN_SECOND;

use crate::common::error::StrategistError;

pub mod base_client;
pub mod errors;
pub mod grpc_client;
pub mod signing_client;
pub mod wasm_client;

pub(crate) type CosmosServiceClient<T> =
    cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient<T>;
pub(crate) type WasmQueryClient<T> =
    cosmrs::proto::cosmwasm::wasm::v1::query_client::QueryClient<T>;
pub(crate) type BankQueryClient<T> =
    cosmrs::proto::cosmos::bank::v1beta1::query_client::QueryClient<T>;

pub struct ProtoTimestamp(cosmos_sdk_proto::Timestamp);

impl ProtoTimestamp {
    pub fn extend_by_seconds(&mut self, seconds: u64) -> Result<(), StrategistError> {
        let seconds = i64::try_from(seconds)?;
        self.0.seconds += seconds;
        Ok(())
    }

    pub fn to_nanos(&self) -> Result<u64, StrategistError> {
        let current_seconds = u64::try_from(self.0.seconds)?;
        let current_nanos = u64::try_from(self.0.nanos)?;

        current_seconds
            .checked_mul(NANOS_IN_SECOND)
            .ok_or_else(|| {
                StrategistError::QueryError("failed to convert seconds to nanos".to_string())
            })?
            .checked_add(current_nanos)
            .ok_or_else(|| StrategistError::QueryError("failed to add current nanos".to_string()))
    }
}

impl From<cosmos_sdk_proto::Timestamp> for ProtoTimestamp {
    fn from(ts: cosmos_sdk_proto::Timestamp) -> Self {
        ProtoTimestamp(ts)
    }
}
