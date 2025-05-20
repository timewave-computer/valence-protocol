use std::fmt::Display;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::ensure;
use neutron_std::types::neutron::util::precdec::PrecDec;
use valence_library_utils::error::LibraryError;

#[cw_serde]
pub struct PrecDecimalRange {
    pub min: PrecDec,
    pub max: PrecDec,
}

impl Display for PrecDecimalRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.min, self.max)
    }
}

impl PrecDecimalRange {
    /// validates that given `PrecDec` value is contained in the range
    pub fn ensure_contains(&self, val: PrecDec) -> Result<(), LibraryError> {
        ensure!(
            val.ge(&self.min) && val.lt(&self.max),
            LibraryError::ExecutionError(format!("expected range: {self}, got: {val}"))
        );
        Ok(())
    }
}
