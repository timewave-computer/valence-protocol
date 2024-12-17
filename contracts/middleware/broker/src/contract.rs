use std::{collections::BTreeMap, str::FromStr};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::bindings::types::{InterchainQueryResult, KVKey};
use semver::Version;
use valence_middleware_utils::{
    type_registry::types::{NativeTypeWrapper, RegistryQueryMsg},
    MiddlewareError,
};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TypeRegistry},
    state::{ACTIVE_REGISTRIES, LATEST},
};

// version info for migration info
const _CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, MiddlewareError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, MiddlewareError> {
    match msg {
        ExecuteMsg::SetRegistry { version, address } => {
            try_add_new_registry(deps, version, address)
        }
    }
}

fn try_add_new_registry(
    deps: DepsMut,
    version: String,
    addr: String,
) -> Result<Response, MiddlewareError> {
    let addr = deps.api.addr_validate(&addr)?;
    let version = Version::from_str(&version)?;

    let type_registry = TypeRegistry {
        registry_address: addr,
        version: version.to_string(),
    };

    // TODO: likely here we will need to "couple" the new type
    // with the previous one to know the type update route

    ACTIVE_REGISTRIES.save(deps.storage, version.to_string(), &type_registry)?;
    LATEST.save(deps.storage, &version.to_string())?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // if version is specified, we use that. otherwise default to latest.
    let target_version = match msg.registry_version {
        Some(version) => version,
        None => LATEST.load(deps.storage)?,
    };
    // load the target registry
    let registry = ACTIVE_REGISTRIES.load(deps.storage, target_version)?;

    match msg.query {
        RegistryQueryMsg::ReconstructProto {
            query_id,
            icq_result,
        } => try_decode_proto(
            deps,
            registry.registry_address.to_string(),
            query_id,
            icq_result,
        ),
        RegistryQueryMsg::KVKey { type_id, params } => {
            try_get_kv_key(deps, registry.registry_address.to_string(), type_id, params)
        }
        RegistryQueryMsg::ToCanonical { type_url, binary } => try_to_canonical(),
        RegistryQueryMsg::FromCanonical { obj } => try_from_canonical(),
    }
}

fn try_decode_proto(
    deps: Deps,
    registry: String,
    query_id: String,
    icq_result: InterchainQueryResult,
) -> StdResult<Binary> {
    let resp: NativeTypeWrapper = deps.querier.query_wasm_smart(
        registry,
        &RegistryQueryMsg::ReconstructProto {
            query_id,
            icq_result,
        },
    )?;

    to_json_binary(&resp)
}

fn try_get_kv_key(
    deps: Deps,
    registry: String,
    type_id: String,
    params: BTreeMap<String, Binary>,
) -> StdResult<Binary> {
    let response: KVKey = deps
        .querier
        .query_wasm_smart(registry, &RegistryQueryMsg::KVKey { type_id, params })?;

    println!("[broker] response kv key: {:?}", response);

    to_json_binary(&response)
}

fn try_to_canonical() -> StdResult<Binary> {
    Ok(Binary::new("a".as_bytes().to_vec()))
}

fn try_from_canonical() -> StdResult<Binary> {
    Ok(Binary::new("a".as_bytes().to_vec()))
}
