#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply,
    Response, StdError, StdResult, SubMsg,
};
use neutron_sdk::{
    bindings::{
        msg::{MsgRegisterInterchainQueryResponse, NeutronMsg},
        query::NeutronQuery,
    },
    interchain_queries::queries::get_raw_interchain_query_result,
    sudo::msg::SudoMsg,
};
use serde_json::Value;
use valence_icq_lib_utils::{PendingQueryIdConfig, QueryMsg as DomainRegistryQueryMsg};
use valence_icq_lib_utils::{
    QueryReconstructionResponse, QueryRegistrationInfoRequest as DomainRegistryQueryRequest,
};

use valence_icq_lib_utils::QueryRegistrationInfoResponse;
use valence_library_utils::error::LibraryError;

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
            type_registry,
            module,
            query,
        } => register_kv_query(deps, type_registry, module, query),
    }
}

fn register_kv_query(
    deps: DepsMut,
    type_registry: String,
    module: String,
    query: serde_json::Map<String, Value>,
) -> Result<Response<NeutronMsg>, LibraryError> {
    let query_registration_resp: QueryRegistrationInfoResponse = deps.querier.query_wasm_smart(
        type_registry.to_string(),
        &DomainRegistryQueryMsg::GetRegistrationConfig(DomainRegistryQueryRequest {
            module,
            params: query,
        }),
    )?;

    let query_cfg = PendingQueryIdConfig {
        associated_domain_registry: type_registry,
        query_type: query_registration_resp.query_type.clone(),
    };

    // here the key is set to the resp.reply_id just to get to the reply handler.
    // it will get overriden by the actual query id in the reply handler.
    ASSOCIATED_QUERY_IDS.save(deps.storage, query_registration_resp.reply_id, &query_cfg)?;

    // fire registration in a submsg to get the registered query id back
    let submsg = SubMsg::reply_on_success(
        query_registration_resp.registration_msg.clone(),
        query_registration_resp.reply_id,
    );

    Ok(Response::default().add_submessage(submsg).add_attribute(
        "query_registration_response".to_string(),
        to_json_string(&query_registration_resp)?,
    ))
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

    let json_response: QueryReconstructionResponse = deps.querier.query_wasm_smart(
        pending_query_config.associated_domain_registry,
        &DomainRegistryQueryMsg::ReconstructQuery(
            valence_icq_lib_utils::QueryReconstructionRequest {
                icq_result: registered_query_result.result,
                query_type: pending_query_config.query_type,
            },
        ),
    )?;

    QUERY_RESULTS.save(deps.storage, query_id, &json_response.json_value)?;

    Ok(Response::new().add_attribute("query_result", json_response.json_value.to_string()))
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
