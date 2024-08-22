use cosmwasm_std::{Addr, Storage};

use crate::{error::ServiceError, state::PROCESSOR};

pub fn assert_processor(store: &dyn Storage, sender: &Addr) -> Result<(), ServiceError> {
    let processor = PROCESSOR.load(store)?;
    if sender != processor {
        return Err(ServiceError::NotProcessor);
    }
    Ok(())
}
