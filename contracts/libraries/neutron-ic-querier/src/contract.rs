#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    QueryRequest, Reply, Response, StdError, StdResult, SubMsg, Uint64, WasmMsg,
};
use cw_utils::must_pay;
use neutron_sdk::{
    bindings::{
        msg::{MsgRegisterInterchainQueryResponse, NeutronMsg},
        query::{NeutronQuery, QueryRegisteredQueryResponse},
        types::KVKey,
    },
    interchain_queries::{queries::get_raw_interchain_query_result, types::QueryType},
    sudo::msg::SudoMsg,
};

use valence_library_utils::{
    error::LibraryError,
    msg::{ExecuteMsg, InstantiateMsg},
};
use valence_middleware_utils::type_registry::types::{
    NativeTypeWrapper, RegistryQueryMsg, ValenceType,
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type QueryDeps<'a> = Deps<'a, NeutronQuery>;
pub type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<LibraryConfig>,
) -> Result<Response, LibraryError> {
    valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: ExecuteDeps,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response<NeutronMsg>, LibraryError> {
    valence_library_base::execute(deps, env, info, msg, process_function, update_config)
}

pub fn update_config(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    new_config: LibraryConfigUpdate,
) -> Result<(), LibraryError> {
    new_config.update_config(deps)
}

pub fn process_function(
    deps: ExecuteDeps,
    _env: Env,
    info: MessageInfo,
    msg: FunctionMsgs,
    cfg: Config,
) -> Result<Response<NeutronMsg>, LibraryError> {
    match msg {
        FunctionMsgs::RegisterKvQuery { target_query } => {
            register_kv_query(deps, info, cfg, target_query)
        }
        FunctionMsgs::DeregisterKvQuery { query_id } => deregister_kv_query(deps, info, query_id),
    }
}

fn deregister_kv_query(
    deps: ExecuteDeps,
    info: MessageInfo,
    query_id: u64,
) -> Result<Response<NeutronMsg>, LibraryError> {
    // remove the associated query entry
    let mut config: Config = valence_library_base::load_config(deps.storage)?;
    config.registered_queries.remove(&query_id);
    valence_library_base::save_config(deps.storage, &config)?;

    let query_removal_msg = NeutronMsg::remove_interchain_query(query_id);

    let registered_query: QueryRegisteredQueryResponse = deps
        .querier
        .query(&NeutronQuery::RegisteredInterchainQuery { query_id }.into())?;

    let transfer_escrow_msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: registered_query.registered_query.deposit,
    };

    Ok(Response::new()
        .add_message(query_removal_msg)
        .add_message(transfer_escrow_msg))
}

fn register_kv_query(
    deps: ExecuteDeps,
    info: MessageInfo,
    cfg: Config,
    target_query: String,
) -> Result<Response<NeutronMsg>, LibraryError> {
    let query_definition = cfg
        .query_definitions
        .get(&target_query)
        .ok_or(LibraryError::Std(StdError::generic_err(
            "no query definition for key",
        )))?;

    // query the icq registration fee from `interchainqueries` module
    let icq_registration_fee = query_icq_registration_fee(deps.as_ref().into_empty().querier)?;

    // get the amount of fee denom paid by the sender
    let paid_fee_denom_amount = must_pay(&info, &icq_registration_fee.denom)
        .map_err(|_| StdError::generic_err("sender must pay icq registration fee"))?;

    ensure!(
        paid_fee_denom_amount >= icq_registration_fee.amount,
        StdError::generic_err("insufficient icq registration fee amount")
    );

    let query_kv_key: KVKey = deps.querier.query_wasm_smart(
        cfg.querier_config.broker_addr.to_string(),
        &valence_middleware_broker::msg::QueryMsg {
            registry_version: query_definition.registry_version.clone(),
            query: RegistryQueryMsg::KVKey {
                type_id: query_definition.type_url.to_string(),
                params: query_definition.params.clone(),
            },
        },
    )?;

    let kv_registration_msg = NeutronMsg::RegisterInterchainQuery {
        query_type: QueryType::KV.into(),
        keys: vec![query_kv_key],
        transactions_filter: String::new(),
        connection_id: cfg.querier_config.connection_id.to_string(),
        update_period: query_definition.update_period.u64(),
    };

    // we load the current library config to get the nonce for submsg
    // reply processing
    let mut config: Config = valence_library_base::load_config(deps.storage)?;

    // get the nonce of the query to be registered to serve as a temporary identifier
    let nonce = config.pending_query_registrations.len() as u64;
    config
        .pending_query_registrations
        .insert(nonce, target_query);

    // save the config
    valence_library_base::save_config(deps.storage, &config)?;

    // fire registration in a submsg to get the registered query id back
    let submsg = SubMsg::reply_on_success(kv_registration_msg, nonce);

    Ok(Response::default().add_submessage(submsg))
}

fn query_icq_registration_fee(querier: QuerierWrapper) -> Result<Coin, StdError> {
    #[cosmwasm_schema::cw_serde]
    struct Params {
        pub query_submit_timeout: Uint64,
        pub query_deposit: Vec<Coin>,
        pub tx_query_removal_limit: Uint64,
    }
    #[cosmwasm_schema::cw_serde]
    struct QueryParamsResponse {
        pub params: Params,
    }

    let query_request = QueryRequest::Stargate {
        path: "/neutron.interchainqueries.Query/Params".to_owned(),
        data: Binary::from(vec![]),
    };

    let res: QueryParamsResponse = querier.query(&query_request)?;

    match res.params.query_deposit.len() {
        1 => Ok(res.params.query_deposit[0].clone()),
        _ => Err(StdError::generic_err(
            "query deposit response must contain one token",
        )),
    }
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

    let cfg: Config = valence_library_base::load_config(deps.storage)?;

    let target_query_identifier = cfg
        .registered_queries
        .get(&query_id)
        .ok_or_else(|| StdError::generic_err("no active query found for the given query id"))?;

    let target_query_definition = cfg
        .query_definitions
        .get(target_query_identifier)
        .ok_or_else(|| StdError::generic_err("no query definition found for the given query id"))?;

    let proto_reconstruction_response: NativeTypeWrapper = deps.querier.query_wasm_smart(
        cfg.querier_config.broker_addr.to_string(),
        &valence_middleware_broker::msg::QueryMsg {
            registry_version: target_query_definition.registry_version.clone(),
            query: RegistryQueryMsg::ReconstructProto {
                type_id: target_query_definition.type_url.to_string(),
                icq_result: registered_query_result.result,
            },
        },
    )?;

    let valence_canonical_response: ValenceType = deps.querier.query_wasm_smart(
        cfg.querier_config.broker_addr.to_string(),
        &valence_middleware_broker::msg::QueryMsg {
            registry_version: target_query_definition.registry_version.clone(),
            query: RegistryQueryMsg::ToCanonical {
                type_url: target_query_definition.type_url.to_string(),
                binary: proto_reconstruction_response.binary,
            },
        },
    )?;

    let storage_acc_write_msg = WasmMsg::Execute {
        contract_addr: cfg.storage_acc_addr.to_string(),
        msg: to_json_binary(
            &valence_storage_account::msg::ExecuteMsg::StoreValenceType {
                key: target_query_identifier.to_string(),
                variant: valence_canonical_response.clone(),
            },
        )?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(storage_acc_write_msg)
        .add_attribute(
            "query_result",
            to_json_binary(&valence_canonical_response)?.to_string(),
        ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, msg: Reply) -> StdResult<Response> {
    try_associate_registered_query_id(deps, msg)
}

fn try_associate_registered_query_id(deps: DepsMut, reply: Reply) -> StdResult<Response> {
    let submsg_response = reply.result.into_result().map_err(StdError::generic_err)?;

    // TODO: revisit this once we enable cw2.0 as .data is deprecated
    #[allow(deprecated)]
    let binary = submsg_response
        .data
        .ok_or_else(|| StdError::generic_err("no data in reply"))?;

    let icq_registration_response: MsgRegisterInterchainQueryResponse =
        serde_json_wasm::from_slice(binary.as_slice())
            .map_err(|e| StdError::generic_err(e.to_string()))?;

    let mut config: Config = valence_library_base::load_config(deps.storage)?;

    // we remove the pending query registration entry with this submsg reply
    // id as the key which should return the target query identifier
    match config.pending_query_registrations.remove(&reply.id) {
        Some(target_query_id) => {
            // associate the assigned `interchainqueries` query_id with the
            // internal target query id
            config
                .registered_queries
                .insert(icq_registration_response.id, target_query_id);

            valence_library_base::save_config(deps.storage, &config)?;

            Ok(Response::default())
        }
        None => Err(StdError::generic_err(
            "no pending query registration found for the given submsg reply id",
        )),
    }
}
