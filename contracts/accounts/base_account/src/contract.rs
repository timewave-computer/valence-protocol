#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use neutron_sdk::bindings::msg::NeutronMsg;
use valence_account_utils::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use crate::state::APPROVED_SERVICES;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.admin))?;

    msg.approved_services.iter().try_for_each(|service| {
        APPROVED_SERVICES.save(deps.storage, deps.api.addr_validate(service)?, &Empty {})
    })?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    match msg {
        ExecuteMsg::ApproveService { service } => execute::approve_service(deps, info, service),
        ExecuteMsg::RemoveService { service } => execute::remove_service(deps, info, service),
        ExecuteMsg::ExecuteMsg { msgs } => execute::execute_msg(deps, info, msgs),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
    }
}

mod execute {
    use cosmwasm_std::{CosmosMsg, DepsMut, Empty, Env, MessageInfo, Response};
    use neutron_sdk::bindings::msg::NeutronMsg;
    use valence_account_utils::error::{ContractError, UnauthorizedReason};

    use crate::state::APPROVED_SERVICES;

    pub fn approve_service(
        deps: DepsMut,
        info: MessageInfo,
        service: String,
    ) -> Result<Response<NeutronMsg>, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let service_addr = deps.api.addr_validate(&service)?;
        APPROVED_SERVICES.save(deps.storage, service_addr.clone(), &Empty {})?;

        Ok(Response::new()
            .add_attribute("method", "approve_service")
            .add_attribute("service", service_addr))
    }

    pub fn remove_service(
        deps: DepsMut,
        info: MessageInfo,
        service: String,
    ) -> Result<Response<NeutronMsg>, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let service_addr = deps.api.addr_validate(&service)?;
        APPROVED_SERVICES.remove(deps.storage, service_addr.clone());

        Ok(Response::new()
            .add_attribute("method", "remove_service")
            .add_attribute("service", service_addr))
    }

    pub fn execute_msg(
        deps: DepsMut,
        info: MessageInfo,
        msgs: Vec<CosmosMsg<NeutronMsg>>,
    ) -> Result<Response<NeutronMsg>, ContractError> {
        // If not admin, check if it's an approved service
        if !cw_ownable::is_owner(deps.storage, &info.sender)?
            && !APPROVED_SERVICES.has(deps.storage, info.sender.clone())
        {
            return Err(ContractError::Unauthorized(
                UnauthorizedReason::NotAdminOrApprovedService,
            ));
        }

        // Execute the message
        Ok(Response::new()
            .add_messages(msgs)
            .add_attribute("method", "execute_msg")
            .add_attribute("sender", info.sender))
    }

    pub fn update_ownership(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        action: cw_ownable::Action,
    ) -> Result<Response<NeutronMsg>, ContractError> {
        let result = cw_ownable::update_ownership(deps, &env.block, &info.sender, action.clone())?;
        Ok(Response::default()
            .add_attribute("method", "update_ownership")
            .add_attribute("action", format!("{:?}", action))
            .add_attribute("result", format!("{:?}", result)))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::ListApprovedServices {} => {
            let services = APPROVED_SERVICES
                .keys(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;
            to_json_binary(&services)
        }
    }
}
