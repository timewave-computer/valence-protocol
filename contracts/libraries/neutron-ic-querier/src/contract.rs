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
    interchain_queries::{queries::get_raw_interchain_query_result, types::QueryPayload},
    sudo::msg::SudoMsg,
};

use serde_json_wasm::from_slice;
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
        FunctionMsgs::DeregisterKvQuery { target_query } => {
            deregister_kv_query(deps, info, cfg, target_query)
        }
    }
}

fn deregister_kv_query(
    deps: ExecuteDeps,
    info: MessageInfo,
    mut cfg: Config,
    target_query: String,
) -> Result<Response<NeutronMsg>, LibraryError> {
    // load the target query and get its asigned query id
    let query_definition = cfg
        .query_definitions
        .get_mut(&target_query)
        .ok_or(StdError::generic_err("query definition not found"))?;
    let query_id = query_definition
        .query_id
        .ok_or(StdError::generic_err("query is not registered"))?;

    // clear the active query id from the query definition and remove
    // the registered query entry from the library config
    query_definition.query_id = None;
    cfg.registered_queries.remove(&query_id);

    valence_library_base::save_config(deps.storage, &cfg)?;

    // build the query removal message
    let query_removal_msg = NeutronMsg::remove_interchain_query(query_id);

    // get the ic query to be removed in order to find the escrowed deposit
    let registered_query: QueryRegisteredQueryResponse = deps
        .querier
        .query(&NeutronQuery::RegisteredInterchainQuery { query_id }.into())?;

    // transfer the recovered deposit back to the sender
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
    mut cfg: Config,
    target_query: String,
) -> Result<Response<NeutronMsg>, LibraryError> {
    // lookup the target query definition in the library config
    let query_definition = cfg
        .query_definitions
        .get(&target_query)
        .ok_or(StdError::generic_err("query definition not found"))?;

    // query the icq registration fee from `interchainqueries` module
    let icq_registration_fee = query_icq_registration_fee(deps.as_ref().into_empty().querier)?;

    // this is expected to just contain a single coin (untrn), but we respect
    // the response type being a vector and assert all potential coins
    for fee_coin in icq_registration_fee {
        // get the amount of fee denom paid by the sender
        let paid_fee_denom_amount = must_pay(&info, &fee_coin.denom)
            .map_err(|_| StdError::generic_err("sender must pay icq registration fee"))?;

        ensure!(
            paid_fee_denom_amount >= fee_coin.amount,
            StdError::generic_err("insufficient icq registration fee amount")
        );
    }

    // query the broker for the KV key to be registered
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

    let kv_registration_msg = NeutronMsg::register_interchain_query(
        QueryPayload::KV(vec![query_kv_key]),
        cfg.querier_config.connection_id.to_string(),
        query_definition.update_period.u64(),
    )
    .map_err(|e| StdError::generic_err(e.to_string()))?;

    // get the nonce of the query to be registered to serve as a temporary identifier
    let nonce = cfg.pending_query_registrations.len() as u64;

    // write the pending query registration to the config in order to
    // later associate it in the reply handler
    cfg.pending_query_registrations.insert(nonce, target_query);
    valence_library_base::save_config(deps.storage, &cfg)?;

    // fire registration in a submsg to get the registered query id back
    let submsg = SubMsg::reply_on_success(kv_registration_msg, nonce);

    Ok(Response::default().add_submessage(submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, _env: Env, msg: SudoMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        // this is triggered by the ICQ relayer delivering the query result
        SudoMsg::KVQueryResult { query_id } => handle_sudo_kv_query_result(deps, query_id),
        _ => Ok(Response::default()),
    }
}

/// looks up the delivered query_id result from the `interchainqueries` module
/// and processes it to store the canonical result in the storage account
fn handle_sudo_kv_query_result(
    deps: ExecuteDeps,
    query_id: u64,
) -> StdResult<Response<NeutronMsg>> {
    let registered_query_result = get_raw_interchain_query_result(deps.as_ref(), query_id)
        .map_err(|_| StdError::generic_err("failed to get the raw icq result"))?;

    // as this call bypasses the valence_library_base, library config is not in scope.
    // we load it manually.
    let cfg: Config = valence_library_base::load_config(deps.storage)?;

    // lookup the query definition id
    let target_query_identifier = cfg
        .registered_queries
        .get(&query_id)
        .ok_or_else(|| StdError::generic_err("no registered query found for the given query id"))?;
    let target_query_definition = cfg
        .query_definitions
        .get(target_query_identifier)
        .ok_or_else(|| StdError::generic_err("no query definition found for the given query id"))?;

    // call into the broker to deserialize the proto result into a native type (b64 encoded)
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

    // call into the broker to canonicalize the native type into a valence type
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

    // write the resulting canonical response to the storage account under the target query id
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

    Ok(Response::new().add_message(storage_acc_write_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, msg: Reply) -> StdResult<Response> {
    let submsg_response = msg.result.into_result().map_err(StdError::generic_err)?;

    // TODO: revisit this once we enable cw2.0 as .data is deprecated
    #[allow(deprecated)]
    let binary = submsg_response
        .data
        .ok_or_else(|| StdError::generic_err("no data in reply"))?;

    let icq_registration_response: MsgRegisterInterchainQueryResponse =
        from_slice(binary.as_slice()).map_err(|e| StdError::generic_err(e.to_string()))?;

    // as this call bypasses the valence_library_base, library config is not in scope.
    // we load it manually.
    let mut config: Config = valence_library_base::load_config(deps.storage)?;

    // we remove the pending query registration entry with this submsg reply
    // id as the key which should return the target query identifier
    match config.pending_query_registrations.remove(&msg.id) {
        Some(target_query_id) => {
            let query_definition = config
                .query_definitions
                .get_mut(&target_query_id)
                .ok_or_else(|| StdError::generic_err("query definition not found"))?;
            // set the active query id on the query definition
            query_definition.query_id = Some(icq_registration_response.id);

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

fn query_icq_registration_fee(querier: QuerierWrapper) -> Result<Vec<Coin>, StdError> {
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

    #[allow(deprecated)]
    let query_request = QueryRequest::Stargate {
        path: "/neutron.interchainqueries.Query/Params".to_owned(),
        data: Binary::from(vec![]),
    };

    let res: QueryParamsResponse = querier.query(&query_request)?;

    Ok(res.params.query_deposit)
}
