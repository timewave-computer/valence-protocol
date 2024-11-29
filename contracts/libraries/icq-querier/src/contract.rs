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
    interchain_queries::{queries::get_raw_interchain_query_result, types::KVReconstruct},
    sudo::msg::SudoMsg,
};
use prost::Message;
use valence_icq_lib_utils::QueryMsg as DomainRegistryQueryMsg;
use valence_icq_lib_utils::QueryRegistrationInfoRequest as DomainRegistryQueryRequest;

use valence_icq_lib_utils::QueryRegistrationInfoResponse;
use valence_library_utils::error::LibraryError;

use crate::{
    error::ContractError,
    msg::{
        BankResultTypes, Config, FunctionMsgs, GammResultTypes, InstantiateMsg, LibraryConfig,
        QueryMsg, QueryResult,
    },
    state::{
        PendingQueryIdConfig, ASSOCIATED_QUERY_IDS, LOGS, QUERY_REGISTRATION_REPLY_IDS,
        QUERY_RESULTS,
    },
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
    // valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
) -> Result<Response<NeutronMsg>, LibraryError> {
    // valence_library_base::execute(deps, env, info, msg, process_function, update_config)
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
    query: String,
) -> Result<Response<NeutronMsg>, LibraryError> {
    let query_registration_resp: QueryRegistrationInfoResponse = deps.querier.query_wasm_smart(
        type_registry.to_string(),
        &DomainRegistryQueryMsg::GetRegistrationConfig(DomainRegistryQueryRequest {
            module: module.to_string(),
            query,
        }),
    )?;

    let query_type = match module.as_str() {
        "gamm" => QueryResult::Gamm {
            result_type: GammResultTypes::Pool,
        },
        "bank" => QueryResult::Bank {
            result_type: BankResultTypes::AccountDenomBalance,
        },
        _ => return Err(ContractError::UnsupportedModule(module).into()),
    };
    let query_cfg = PendingQueryIdConfig {
        associated_domain_registry: type_registry,
        query_type,
    };

    QUERY_REGISTRATION_REPLY_IDS.save(
        deps.storage,
        query_registration_resp.reply_id,
        &query_cfg,
    )?;

    // fire registration in a submsg to get the registered query id back
    let submsg = SubMsg::reply_on_success(
        query_registration_resp.registration_msg,
        query_registration_resp.reply_id,
    );

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
        QueryMsg::Logs {} => {
            let mut resp = vec![];
            for entry in LOGS.range(deps.storage, None, None, Order::Ascending) {
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
pub fn sudo(deps: ExecuteDeps, env: Env, msg: SudoMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        SudoMsg::KVQueryResult { query_id } => handle_sudo_kv_query_result(deps, query_id),
        _ => {
            LOGS.save(
                deps.storage,
                format!("sudo_catchall_handler-{}", env.block.height).to_string(),
                &to_json_string(&msg)?,
            )?;
            Ok(Response::default())
        }
    }
}

fn handle_sudo_kv_query_result(
    deps: ExecuteDeps,
    query_id: u64,
) -> StdResult<Response<NeutronMsg>> {
    let registered_query_result = get_raw_interchain_query_result(deps.as_ref(), query_id)
        .map_err(|_| StdError::generic_err("failed to get the raw icq result"))?;

    let pending_query_config = ASSOCIATED_QUERY_IDS.load(deps.storage, query_id)?;
    let query_result_str = match pending_query_config.query_type {
        QueryResult::Gamm { result_type } => match result_type {
            GammResultTypes::Pool => {
                let any_msg: osmosis_std::shim::Any = osmosis_std::shim::Any::decode(
                    registered_query_result.result.kv_results[0]
                        .value
                        .as_slice(),
                )
                .unwrap();
                assert_eq!(any_msg.type_url, "/osmosis.gamm.v1beta1.Pool");

                let osmo_pool: osmosis_std::types::osmosis::gamm::v1beta1::Pool =
                    any_msg.try_into().unwrap();

                to_json_string(&osmo_pool).unwrap()
            }
        },
        QueryResult::Bank { result_type } => match result_type {
            BankResultTypes::AccountDenomBalance => {
                let balances: neutron_sdk::interchain_queries::v047::types::Balances =
                    KVReconstruct::reconstruct(&registered_query_result.result.kv_results).unwrap();

                to_json_string(&balances).unwrap()
            }
        },
    };

    let json_response: serde_json::Value = serde_json::from_str(&query_result_str).unwrap();

    QUERY_RESULTS.save(deps.storage, query_id, &json_response)?;

    Ok(Response::new().add_attribute("query_result", json_response.to_string()))
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

    LOGS.save(
        deps.storage,
        format!("registered_query_type_{}", reply.id),
        &reply.id.to_string(),
    )?;

    let query_cfg = QUERY_REGISTRATION_REPLY_IDS.load(deps.storage, reply.id)?;

    ASSOCIATED_QUERY_IDS.save(deps.storage, resp.id, &query_cfg)?;

    QUERY_REGISTRATION_REPLY_IDS.remove(deps.storage, reply.id);

    Ok(Response::default())
}
