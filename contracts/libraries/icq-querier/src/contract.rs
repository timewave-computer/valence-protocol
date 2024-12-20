use std::collections::BTreeMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response, StdError,
    StdResult, SubMsg,
};
use neutron_sdk::{
    bindings::{
        msg::{MsgRegisterInterchainQueryResponse, NeutronMsg},
        query::NeutronQuery,
        types::KVKey,
    },
    interchain_queries::{queries::get_raw_interchain_query_result, types::QueryType},
    sudo::msg::SudoMsg,
};
use valence_icq_lib_utils::PendingQueryIdConfig;

use valence_library_utils::error::LibraryError;
use valence_middleware_utils::type_registry::types::{NativeTypeWrapper, RegistryQueryMsg};

use crate::{
    msg::{Config, FunctionMsgs, InstantiateMsg, LibraryConfig, QueryMsg},
    state::{ASSOCIATED_QUERY_IDS, QUERY_RESULTS},
};

// version info for migration info
const _CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type QueryDeps<'a> = Deps<'a, NeutronQuery>;
pub type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response<NeutronMsg>, LibraryError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
) -> Result<Response<NeutronMsg>, LibraryError> {
    match msg {
        FunctionMsgs::RegisterKvQuery {
            broker_addr,
            type_id,
            connection_id,
            params,
        } => register_kv_query(deps, broker_addr, type_id, connection_id, params),
    }
}

fn register_kv_query(
    deps: DepsMut,
    broker_addr: String,
    type_id: String,
    connection_id: String,
    params: BTreeMap<String, Binary>,
) -> Result<Response<NeutronMsg>, LibraryError> {
    let query_kv_key: KVKey = deps.querier.query_wasm_smart(
        broker_addr.to_string(),
        &valence_middleware_broker::msg::QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::KVKey {
                type_id: type_id.to_string(),
                params,
            },
        },
    )?;

    let kv_registration_msg = NeutronMsg::RegisterInterchainQuery {
        query_type: QueryType::KV.into(),
        keys: vec![query_kv_key],
        transactions_filter: String::new(),
        connection_id,
        update_period: 5,
    };

    // here the key is set to the resp.reply_id just to get to the reply handler.
    // it will get overriden by the actual query id in the reply handler.
    ASSOCIATED_QUERY_IDS.save(
        deps.storage,
        1,
        &PendingQueryIdConfig {
            associated_domain_registry: broker_addr,
            query_type: type_id,
        },
    )?;

    // fire registration in a submsg to get the registered query id back
    let submsg = SubMsg::reply_on_success(kv_registration_msg, 1);

    Ok(Response::default().add_submessage(submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_library_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_library_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetLibraryConfig {} => {
            let config: Config = valence_library_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawLibraryConfig {} => {
            let raw_config: LibraryConfig =
                valence_library_utils::raw_config::query_raw_library_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
        QueryMsg::RegisteredQueries {} => {
            let mut resp = vec![];
            for entry in ASSOCIATED_QUERY_IDS.range(deps.storage, None, None, Order::Ascending) {
                resp.push(entry?);
            }
            to_json_binary(&resp)
        }
        QueryMsg::QueryResults {} => {
            let mut resp = vec![];
            for entry in QUERY_RESULTS.range(deps.storage, None, None, Order::Ascending) {
                resp.push(entry?);
            }
            to_json_binary(&resp)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, _env: Env, msg: SudoMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        SudoMsg::KVQueryResult { query_id } => handle_sudo_kv_query_result(deps, query_id),
        _ => Ok(Response::default()),
    }
}

fn handle_sudo_kv_query_result(
    deps: ExecuteDeps,
    query_id: u64,
) -> StdResult<Response<NeutronMsg>> {
    let registered_query_result = get_raw_interchain_query_result(deps.as_ref(), query_id)
        .map_err(|_| StdError::generic_err("failed to get the raw icq result"))?;

    let pending_query_config = ASSOCIATED_QUERY_IDS.load(deps.storage, query_id)?;

    let reconstruction_response: NativeTypeWrapper = deps.querier.query_wasm_smart(
        pending_query_config.associated_domain_registry,
        &valence_middleware_broker::msg::QueryMsg {
            registry_version: None,
            query: RegistryQueryMsg::ReconstructProto {
                query_id: pending_query_config.query_type,
                icq_result: registered_query_result.result,
            },
        },
    )?;

    QUERY_RESULTS.save(deps.storage, query_id, &reconstruction_response.binary)?;

    Ok(Response::new().add_attribute(
        "query_result",
        to_json_binary(&reconstruction_response)?.to_string(),
    ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, msg: Reply) -> StdResult<Response> {
    try_associate_registered_query_id(deps, msg)
}

fn try_associate_registered_query_id(deps: DepsMut, reply: Reply) -> StdResult<Response> {
    let submsg_response = reply.result.into_result().map_err(StdError::generic_err)?;

    // response.data is deprecated
    // TODO: look into whether it's possible to use the cw2.0 method
    #[allow(deprecated)]
    let binary = submsg_response
        .data
        .ok_or_else(|| StdError::generic_err("no data in reply"))?;

    let resp: MsgRegisterInterchainQueryResponse =
        serde_json_wasm::from_slice(binary.as_slice())
            .map_err(|e| StdError::generic_err(e.to_string()))?;

    let pending_query_config = ASSOCIATED_QUERY_IDS.load(deps.storage, reply.id)?;
    ASSOCIATED_QUERY_IDS.save(deps.storage, resp.id, &pending_query_config)?;
    ASSOCIATED_QUERY_IDS.remove(deps.storage, reply.id);

    Ok(Response::default())
}
