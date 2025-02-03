use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Decimal};

use crate::error::LibraryError;

#[cw_serde]
pub struct AssetData {
    /// Denom of the first asset
    pub asset1: String,
    /// Denom of the second asset
    pub asset2: String,
}

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
    pub fn contains(&self, value: Decimal) -> Result<(), LibraryError> {
        ensure!(
            value >= self.min && value <= self.max,
            LibraryError::ExecutionError("Value is not within the expected range".to_string())
        );
        Ok(())
    }
}
