use cosmwasm_std::{DepsMut, MessageInfo};

use crate::{state::ADMIN, ContractError};

pub fn check_admin(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::NotAdmin);
    }
    Ok(())
}
