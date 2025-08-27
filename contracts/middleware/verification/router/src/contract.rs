#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::ROUTES,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// pagination info for queries
const MAX_PAGE_LIMIT: u32 = 250;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    msg.initial_routes
        .into_iter()
        .try_for_each(|(name, verifier)| {
            ROUTES.save(deps.storage, name, &deps.api.addr_validate(&verifier)?)
        })?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddRoute { name, address } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;

            // Check that the route doesn't exist
            if ROUTES.has(deps.storage, name.clone()) {
                return Err(ContractError::RouteAlreadyExists {});
            }

            let addr = deps.api.addr_validate(&address)?;
            ROUTES.save(deps.storage, name.clone(), &addr)?;

            Ok(Response::new()
                .add_attribute("method", "add_route")
                .add_attribute("name", name)
                .add_attribute("verifier", addr))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetRoute { name } => to_json_binary(&get_route(deps, name)?),
        QueryMsg::GetRoutes { start_after, limit } => {
            to_json_binary(&get_routes(deps, start_after, limit))
        }
        QueryMsg::Verify {
            route,
            vk,
            inputs,
            proof,
            payload,
        } => to_json_binary(&verify(deps, route, vk, inputs, proof, payload)?),
    }
}

fn get_route(deps: Deps, name: String) -> StdResult<Addr> {
    ROUTES.load(deps.storage, name)
}

fn get_routes(deps: Deps, start_after: Option<String>, limit: Option<u32>) -> Vec<(String, Addr)> {
    let limit = limit.unwrap_or(MAX_PAGE_LIMIT).min(MAX_PAGE_LIMIT);
    let start = start_after.map(Bound::exclusive);

    ROUTES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .filter_map(Result::ok)
        .collect()
}

fn verify(
    deps: Deps,
    route: String,
    vk: Binary,
    inputs: Binary,
    proof: Binary,
    payload: Binary,
) -> StdResult<bool> {
    let verifier = get_route(deps, route)?;
    // Query the verifier contract and return the response
    deps.querier.query_wasm_smart(
        verifier,
        &valence_verification_utils::verifier::QueryMsg::Verify {
            vk,
            inputs,
            proof,
            payload,
        },
    )
}
