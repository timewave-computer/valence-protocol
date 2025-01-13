use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Decimal};
use valence_library_utils::error::LibraryError;

#[cw_serde]
pub struct DecimalRange {
    min: Decimal,
    max: Decimal,
}

impl DecimalRange {
    pub fn new(min: Decimal, max: Decimal) -> Self {
        DecimalRange { min, max }
    }
}

impl DecimalRange {
    pub fn is_within_range(&self, value: Decimal) -> Result<(), LibraryError> {
        ensure!(
            value >= self.min && value <= self.max,
            LibraryError::ExecutionError("Value is not within the expected range".to_string())
        );
        Ok(())
    }
}
