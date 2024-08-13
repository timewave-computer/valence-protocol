use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order,
    Response, StdResult, Storage,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner, is_owner};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, OwnerMsg, QueryMsg},
    state::SUB_OWNERS,
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

    if let Some(sub_owners) = msg.sub_owners {
        for sub_owner in sub_owners {
            SUB_OWNERS.save(deps.storage, sub_owner, &Empty {})?;
        }
    }

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
            assert_owner_or_subowner(deps.storage, info.sender)?;
            match sub_owner_msg {}
        }
        ExecuteMsg::UserAction(_) => todo!(),
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
    SUB_OWNERS.save(deps.storage, sub_owner.clone(), &Empty {})?;

    Ok(Response::new()
        .add_attribute("action", "add_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

fn remove_sub_owner(deps: DepsMut, sub_owner: Addr) -> Result<Response, ContractError> {
    SUB_OWNERS.remove(deps.storage, sub_owner.clone());

    Ok(Response::new()
        .add_attribute("action", "remove_sub_owner")
        .add_attribute("sub_owner", sub_owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::SubOwners {} => to_json_binary(&get_sub_owners(deps)?),
    }
}

fn get_sub_owners(deps: Deps) -> StdResult<Vec<Addr>> {
    let sub_owners = SUB_OWNERS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (k, _) = item?;
            Ok(k)
        })
        .collect::<StdResult<Vec<Addr>>>()?;
    Ok(sub_owners)
}

// Helpers

/// Asserts that the caller is the owner or a subowner
fn assert_owner_or_subowner(store: &dyn Storage, address: Addr) -> Result<(), ContractError> {
    if !is_owner(store, &address)? && !SUB_OWNERS.has(store, address) {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}
