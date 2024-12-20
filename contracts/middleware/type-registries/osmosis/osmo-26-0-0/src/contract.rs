use crate::definitions::handle_query;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;
use valence_middleware_utils::type_registry::types::{
    RegistryExecuteMsg, RegistryInstantiateMsg, RegistryQueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: RegistryInstantiateMsg,
) -> Result<Response, StdError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: RegistryExecuteMsg,
) -> Result<Response, StdError> {
    unimplemented!("execute is disabled")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: RegistryQueryMsg) -> StdResult<Binary> {
    println!("[registry] delegating query to macro handler & hoping for the best");
    handle_query(msg)
}
