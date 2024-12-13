use std::collections::BTreeMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    xyk::ValenceXykPool,
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
) -> Result<Response, StdError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Serialize { obj } => try_serialize_obj(obj),
        QueryMsg::Deserialize { type_url, binary } => {
            let deser = try_deserialize_type_url(type_url, binary)?;
            to_json_binary(&deser)
        }
    }
}

fn try_serialize_obj(object: ValenceXykPool) -> StdResult<Binary> {
    Ok(Binary::new("a".as_bytes().to_vec()))
}

fn try_deserialize_type_url(type_url: String, binary: Binary) -> StdResult<ValenceXykPool> {
    Ok(ValenceXykPool {
        assets: vec![],
        total_shares: "hi".to_string(),
        domain_specific_fields: BTreeMap::new(),
    })
}
