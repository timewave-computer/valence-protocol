use std::{collections::BTreeMap, str::FromStr};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::bindings::types::InterchainQueryResult;
use semver::Version;
use valence_middleware_utils::{
    broker::types::{Broker, ExecuteMsg, InstantiateMsg, QueryMsg},
    type_registry::types::RegistryQueryMsg,
    MiddlewareError,
};

use crate::state::{ACTIVE_REGISTRIES, LATEST};

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
        ExecuteMsg::SetLatestRegistry { version, address } => {
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

    let broker = Broker {
        registry_address: addr,
        version: version.to_string(),
    };

    ACTIVE_REGISTRIES.save(deps.storage, version.to_string(), &broker)?;
    LATEST.save(deps.storage, &version.to_string())?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DecodeProto {
            registry_version,
            query_id,
            icq_result,
        } => try_decode_proto(deps, registry_version, query_id, icq_result),
        QueryMsg::GetKVKey {
            registry_version,
            params,
        } => try_get_kv_key(deps, registry_version, params),
        QueryMsg::ToCanonical {} => try_to_canonical(),
        QueryMsg::FromCanonical {} => try_from_canonical(),
    }
}

fn try_decode_proto(
    deps: Deps,
    registry_version: Option<String>,
    query_id: String,
    icq_result: InterchainQueryResult,
) -> StdResult<Binary> {
    let target_registry = get_target_registry(deps, registry_version)?;

    let decoded_result = deps.querier.query_wasm_smart(
        target_registry.registry_address,
        &RegistryQueryMsg::ReconstructProto {
            query_id,
            icq_result,
        },
    )?;

    Ok(decoded_result)
}

fn try_get_kv_key(
    deps: Deps,
    registry_version: Option<String>,
    params: BTreeMap<String, Binary>,
) -> StdResult<Binary> {
    let target_registry = get_target_registry(deps, registry_version)?;

    Ok(Binary::new("a".as_bytes().to_vec()))
}

fn try_to_canonical() -> StdResult<Binary> {
    Ok(Binary::new("a".as_bytes().to_vec()))
}

fn try_from_canonical() -> StdResult<Binary> {
    Ok(Binary::new("a".as_bytes().to_vec()))
}

fn get_target_registry(deps: Deps, version: Option<String>) -> StdResult<Broker> {
    // if version is specified, we use that. otherwise default to latest.
    let target_version = match version {
        Some(version) => version,
        None => LATEST.load(deps.storage)?,
    };
    // load the target registry
    ACTIVE_REGISTRIES.load(deps.storage, target_version)
}
