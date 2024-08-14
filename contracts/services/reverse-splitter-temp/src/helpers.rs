use cosmwasm_std::{DepsMut, MessageInfo};

use crate::{state::PROCESSOR, ContractError};

pub fn is_processor(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    let processor = PROCESSOR.load(deps.storage)?;
    if info.sender != processor {
        return Err(ContractError::NotProcessor);
    }
    Ok(())
}
