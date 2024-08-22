use cosmwasm_std::{Addr, Storage};

use crate::{
    error::{ServiceError, UnauthorizedReason},
    state::PROCESSOR,
};

pub fn assert_processor(store: &dyn Storage, sender: &Addr) -> Result<(), ServiceError> {
    let processor = PROCESSOR.load(store)?;
    if sender != processor {
        return Err(ServiceError::Unauthorized(
            UnauthorizedReason::NotAllowed {},
        ));
    }
    Ok(())
}
