use std::collections::BTreeMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use neutron_sdk::bindings::types::InterchainQueryResult;
use valence_middleware_utils::{
    canonical_types::pools::xyk::ValenceXykPool,
    type_registry::types::{
        RegistryExecuteMsg, RegistryInstantiateMsg, RegistryQueryMsg, ValenceType,
    },
    IcqIntegration,
};

use crate::definitions::{bank_balance::OsmosisBankBalance, gamm_pool::OsmosisXykPool};

// version info for migration info
const _CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: RegistryInstantiateMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: RegistryExecuteMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: RegistryQueryMsg) -> StdResult<Binary> {
    match msg {
        RegistryQueryMsg::Serialize { obj } => try_serialize_obj(obj),
        RegistryQueryMsg::Deserialize { type_url, binary } => {
            let deser = try_deserialize_type_url(type_url, binary)?;
            to_json_binary(&deser)
        }
        RegistryQueryMsg::KVKey { type_id, params } => try_get_kv_key(type_id, params),
        RegistryQueryMsg::ReconstructProto {
            query_id,
            icq_result,
        } => try_reconstruct_proto(query_id, icq_result),
    }
}

fn try_get_kv_key(type_id: String, params: BTreeMap<String, Binary>) -> StdResult<Binary> {
    let kv_key = match type_id.as_str() {
        "gamm_pool" => OsmosisXykPool::get_kv_key(params),
        "bank_balances" => OsmosisBankBalance::get_kv_key(params),
        _ => return Err(StdError::generic_err("unknown type_id")),
    };

    match kv_key {
        Ok(kv) => to_json_binary(&kv),
        Err(_) => Err(StdError::generic_err("failed to read kv key")),
    }
}

fn try_reconstruct_proto(type_id: String, icq_result: InterchainQueryResult) -> StdResult<Binary> {
    let reconstruction_result = match type_id.as_str() {
        "gamm_pool" => OsmosisXykPool::decode_and_reconstruct(type_id, icq_result),
        "bank_balances" => OsmosisBankBalance::decode_and_reconstruct(type_id, icq_result),
        _ => return Err(StdError::generic_err("unknown type_id")),
    };

    match reconstruction_result {
        Ok(res) => to_json_binary(&res),
        Err(_) => Err(StdError::generic_err(
            "failed to reconstruct type from proto",
        )),
    }
}

fn try_serialize_obj(_object: ValenceType) -> StdResult<Binary> {
    Ok(Binary::new("a".as_bytes().to_vec()))
}

fn try_deserialize_type_url(_type_url: String, _binary: Binary) -> StdResult<ValenceType> {
    Ok(ValenceType::XykPool(ValenceXykPool {
        assets: vec![],
        total_shares: "hi".to_string(),
        domain_specific_fields: BTreeMap::new(),
    }))
}
