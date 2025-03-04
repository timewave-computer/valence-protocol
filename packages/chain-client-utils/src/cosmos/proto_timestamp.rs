use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::Header;

use crate::common::error::StrategistError;

pub struct ProtoTimestamp(cosmos_sdk_proto::Timestamp);

pub const NANOS_IN_SECOND: u64 = 1_000_000_000;

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

impl TryFrom<Header> for ProtoTimestamp {
    type Error = StrategistError;

    fn try_from(value: Header) -> Result<Self, Self::Error> {
        let proto_time = value
            .time
            .ok_or_else(|| StrategistError::QueryError("No time in block header".to_string()))?
            .into();

        Ok(proto_time)
    }
}
