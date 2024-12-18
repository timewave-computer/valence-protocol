use std::collections::BTreeMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};
use neutron_sdk::bindings::types::InterchainQueryResult;
use osmosis_std::types::{
    cosmos::bank::v1beta1::QueryBalanceResponse, osmosis::gamm::v1beta1::Pool,
};
use valence_middleware_utils::{
    canonical_types::ValenceTypeAdapter,
    type_registry::types::{
        NativeTypeWrapper, RegistryExecuteMsg, RegistryInstantiateMsg, RegistryQueryMsg,
        ValenceType,
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
        RegistryQueryMsg::FromCanonical { obj } => try_from_canonical(obj),
        RegistryQueryMsg::ToCanonical { type_url, binary } => {
            let canonical = try_to_canonical(type_url, binary)?;
            to_json_binary(&canonical)
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
        Pool::TYPE_URL => OsmosisXykPool::get_kv_key(params),
        QueryBalanceResponse::TYPE_URL => OsmosisBankBalance::get_kv_key(params),
        _ => return Err(StdError::generic_err("unknown type_id")),
    };

    println!("[registry] kv key: {:?}", kv_key);

    match kv_key {
        Ok(kv) => to_json_binary(&kv),
        Err(_) => Err(StdError::generic_err("failed to read kv key")),
    }
}

fn try_reconstruct_proto(type_id: String, icq_result: InterchainQueryResult) -> StdResult<Binary> {
    let reconstruction_result = match type_id.as_str() {
        Pool::TYPE_URL => OsmosisXykPool::decode_and_reconstruct(type_id, icq_result),
        QueryBalanceResponse::TYPE_URL => {
            OsmosisBankBalance::decode_and_reconstruct(type_id, icq_result)
        }
        _ => return Err(StdError::generic_err("unknown type_id")),
    };

    match reconstruction_result {
        Ok(res) => to_json_binary(&NativeTypeWrapper { binary: res }),
        Err(_) => Err(StdError::generic_err(
            "failed to reconstruct type from proto",
        )),
    }
}

fn try_from_canonical(object: ValenceType) -> StdResult<Binary> {
    println!("[registry] converting valence type to binary: {:?}", object);
    match &object {
        ValenceType::XykPool(_) => {
            let obj = OsmosisXykPool::try_from_canonical(object)
                .map_err(|e| StdError::generic_err(e.to_string()))?;
            to_json_binary(&NativeTypeWrapper {
                binary: to_json_binary(&obj)?,
            })
        }
        ValenceType::BankBalance(_) => {
            let obj = OsmosisBankBalance::try_from_canonical(object)
                .map_err(|e| StdError::generic_err(e.to_string()))?;
            to_json_binary(&NativeTypeWrapper {
                binary: to_json_binary(&obj)?,
            })
        }
    }
}

fn try_to_canonical(type_url: String, binary: Binary) -> StdResult<ValenceType> {
    println!("[registry] converting {type_url} binary to canonical valence type");
    let canonical_type = match type_url.as_str() {
        Pool::TYPE_URL => {
            let obj: Pool = from_json(&binary).map_err(|e| StdError::generic_err(e.to_string()))?;

            let native_type = OsmosisXykPool(obj);
            native_type
                .try_to_canonical()
                .map_err(|e| StdError::generic_err(e.to_string()))?
        }
        QueryBalanceResponse::TYPE_URL => {
            let obj: QueryBalanceResponse =
                from_json(&binary).map_err(|e| StdError::generic_err(e.to_string()))?;

            let native_type = OsmosisBankBalance(obj);
            native_type
                .try_to_canonical()
                .map_err(|e| StdError::generic_err(e.to_string()))?
        }
        _ => return Err(StdError::generic_err("unknown type_id")),
    };

    Ok(canonical_type)
}
