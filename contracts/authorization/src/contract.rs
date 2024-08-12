use std::collections::HashSet;

use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Storage,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner, is_owner};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, OwnerMsg, QueryMsg},
    state::{Config, CONFIG},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    initialize_owner(
        deps.storage,
        deps.api,
        Some(
            deps.api
                .addr_validate(msg.owner.unwrap_or(info.sender).as_str())?
                .as_str(),
        ),
    )?;

    let mut config = Config {
        sub_owners: HashSet::new(),
    };
    if let Some(sub_owners) = msg.sub_owners {
        for sub_owner in sub_owners {
            config
                .sub_owners
                .insert(deps.api.addr_validate(sub_owner.as_str())?);
        }
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "instantiate_authorization"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => update_ownership(deps, env, info, action),
        ExecuteMsg::OwnerAction(owner_msg) => {
            assert_owner(deps.storage, &info.sender)?;
            match owner_msg {
                OwnerMsg::AddSubOwner { sub_owner } => add_sub_owner(deps, sub_owner),
                OwnerMsg::RemoveSubOwner { sub_owner } => remove_sub_owner(deps, sub_owner),
            }
        }
        ExecuteMsg::SubOwnerAction(sub_owner_msg) => {
            assert_owner_or_subowner(deps.storage, &info.sender)?;
            match sub_owner_msg {
                // SubOwnerMsg::Execute { contract_addr, msg } => todo!(),
            }
        }
    }
}

fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new().add_attributes(ownership.into_attributes()))
}

fn add_sub_owner(deps: DepsMut, sub_owner: Addr) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.sub_owners.insert(sub_owner.clone());
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "add_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn remove_sub_owner(deps: DepsMut, sub_owner: Addr) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.sub_owners.remove(&sub_owner);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "remove_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::Config {} => to_json_binary(&get_config(deps)?),
    }
}

fn get_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

// Helpers
/// Asserts that the caller is the owner or a subowner
fn assert_owner_or_subowner(store: &dyn Storage, address: &Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(store)?;
    if !is_owner(store, address)? && !config.sub_owners.contains(address) {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}
