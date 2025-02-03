use cosmwasm_std::{Addr, Storage};
use valence_library_utils::error::{LibraryError, UnauthorizedReason};

use crate::state::PROCESSOR;

pub fn assert_processor(store: &dyn Storage, sender: &Addr) -> Result<(), LibraryError> {
    let processor = PROCESSOR.load(store)?;
    if sender != processor {
        return Err(LibraryError::Unauthorized(
            UnauthorizedReason::NotAllowed {},
        ));
    }
    Ok(())
}
