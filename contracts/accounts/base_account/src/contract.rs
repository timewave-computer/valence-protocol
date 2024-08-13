#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::ADMIN;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:base_account";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;
    ADMIN.save(deps.storage, &admin)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::TransferAdmin { new_admin } => execute::transfer_admin(deps, info, new_admin),
        ExecuteMsg::ApproveService { service } => execute::approve_service(deps, info, service),
        ExecuteMsg::RemoveService { service } => execute::remove_service(deps, info, service),
        ExecuteMsg::ExecuteMsg { msgs } => execute::execute_msg(deps, info, msgs),
    }
}

mod execute {
    use cosmwasm_std::{CosmosMsg, DepsMut, MessageInfo, Response};

    use crate::{
        helpers::check_admin,
        state::{ADMIN, APPROVED_SERVICES},
        ContractError,
    };

    pub fn transfer_admin(
        deps: DepsMut,
        info: MessageInfo,
        new_admin: String,
    ) -> Result<Response, ContractError> {
        check_admin(&deps, &info)?;

        let new_admin = deps.api.addr_validate(&new_admin)?;
        ADMIN.save(deps.storage, &new_admin)?;

        Ok(Response::new()
            .add_attribute("method", "transfer_admin")
            .add_attribute("new_admin", new_admin))
    }

    pub fn approve_service(
        deps: DepsMut,
        info: MessageInfo,
        service: String,
    ) -> Result<Response, ContractError> {
        check_admin(&deps, &info)?;

        let service_addr = deps.api.addr_validate(&service)?;
        APPROVED_SERVICES.save(deps.storage, service_addr.clone(), &true)?;

        Ok(Response::new()
            .add_attribute("method", "approve_service")
            .add_attribute("service", service_addr))
    }

    pub fn remove_service(
        deps: DepsMut,
        info: MessageInfo,
        service: String,
    ) -> Result<Response, ContractError> {
        check_admin(&deps, &info)?;

        let service_addr = deps.api.addr_validate(&service)?;
        APPROVED_SERVICES.remove(deps.storage, service_addr.clone());

        Ok(Response::new()
            .add_attribute("method", "remove_service")
            .add_attribute("service", service_addr))
    }

    pub fn execute_msg(
        deps: DepsMut,
        info: MessageInfo,
        msgs: Vec<CosmosMsg>,
    ) -> Result<Response, ContractError> {
        // If not admin, check if it's an approved service
        if check_admin(&deps, &info).is_err() {
            APPROVED_SERVICES
                .load(deps.storage, info.sender.clone())
                .map_err(|_| ContractError::NotAdminOrApprovedService)?;
        };

        // Execute the message
        Ok(Response::new()
            .add_messages(msgs)
            .add_attribute("method", "execute_msg")
            .add_attribute("sender", info.sender))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
