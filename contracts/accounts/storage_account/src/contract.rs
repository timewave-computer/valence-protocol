#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;
use valence_account_utils::{error::ContractError, msg::InstantiateMsg};

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::{APPROVED_LIBRARIES, BLOB_STORE},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.admin))?;

    msg.approved_libraries.iter().try_for_each(|library| {
        APPROVED_LIBRARIES.save(deps.storage, deps.api.addr_validate(library)?, &Empty {})
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
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ApproveLibrary { library } => execute::approve_library(deps, info, library),
        ExecuteMsg::RemoveLibrary { library } => execute::remove_library(deps, info, library),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
        ExecuteMsg::PostBlob { key, value } => execute::try_post_blob(deps, key, value),
    }
}

mod execute {
    use cosmwasm_std::{Binary, DepsMut, Empty, Env, MessageInfo, Response};
    use valence_account_utils::error::ContractError;

    use crate::state::{APPROVED_LIBRARIES, BLOB_STORE};

    pub fn try_post_blob(
        deps: DepsMut,
        key: String,
        value: Binary,
    ) -> Result<Response, ContractError> {
        // store the value
        BLOB_STORE.save(deps.storage, key.clone(), &value)?;

        Ok(Response::new()
            .add_attribute("method", "post_blob")
            .add_attribute("key", key)
            .add_attribute("value", format!("{:?}", value)))
    }

    pub fn approve_library(
        deps: DepsMut,
        info: MessageInfo,
        library: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let library_addr = deps.api.addr_validate(&library)?;
        APPROVED_LIBRARIES.save(deps.storage, library_addr.clone(), &Empty {})?;

        Ok(Response::new()
            .add_attribute("method", "approve_library")
            .add_attribute("library", library_addr))
    }

    pub fn remove_library(
        deps: DepsMut,
        info: MessageInfo,
        library: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let library_addr = deps.api.addr_validate(&library)?;
        APPROVED_LIBRARIES.remove(deps.storage, library_addr.clone());

        Ok(Response::new()
            .add_attribute("method", "remove_library")
            .add_attribute("library", library_addr))
    }

    pub fn update_ownership(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        action: cw_ownable::Action,
    ) -> Result<Response, ContractError> {
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
        QueryMsg::ListApprovedLibraries {} => {
            let libraries = APPROVED_LIBRARIES
                .keys(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;
            to_json_binary(&libraries)
        }
        QueryMsg::Blob { key } => {
            let blob = BLOB_STORE.may_load(deps.storage, key)?;

            match blob {
                Some(val) => to_json_binary(&val),
                None => Err(StdError::generic_err("blob not found")),
            }
        }
    }
}
