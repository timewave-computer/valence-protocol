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
        types::KVKey,
    },
    interchain_queries::{
        helpers::decode_and_convert,
        queries::get_raw_interchain_query_result,
        types::{KVReconstruct, QueryType},
        v047::{helpers::create_account_denom_balance_key, types::BANK_STORE_KEY},
    },
    sudo::msg::SudoMsg,
};
use prost::Message;
use valence_library_utils::error::LibraryError;

use crate::{
    error::ContractError,
    msg::{
        BankResultTypes, Config, FunctionMsgs, GammResultTypes, InstantiateMsg, LibraryConfig,
        LibraryConfigUpdate, QueryMsg, QueryResult,
    },
    state::{ASSOCIATED_QUERY_IDS, LOGS, QUERY_RESULTS},
};

// version info for migration info
const _CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const GAMM_QUERY_REGISTRATION_REPLY_ID: u64 = 31415;
const BANK_QUERY_REGISTRATION_REPLY_ID: u64 = 31416;

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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
) -> Result<Response<NeutronMsg>, LibraryError> {
    // valence_library_base::execute(deps, env, info, msg, process_function, update_config)
    match msg {
        FunctionMsgs::RegisterKvQuery {
            connection_id,
            update_period,
            module,
        } => register_kv_query(connection_id, update_period, module),
    }
}

fn register_kv_query(
    connection_id: String,
    update_period: u64,
    path: String, // aka module, e.g. gamm
) -> Result<Response<NeutronMsg>, LibraryError> {
    let (kv_key, response_code_id) = match path.as_str() {
        "gamm" => {
            let pool_prefix_key: u8 = 0x02;
            let pool_id: u64 = 1;
            let mut pool_access_key = vec![pool_prefix_key];
            pool_access_key.extend_from_slice(&pool_id.to_be_bytes());

            (
                KVKey {
                    path,
                    key: Binary::new(pool_access_key),
                },
                GAMM_QUERY_REGISTRATION_REPLY_ID,
            )
        }
        "bank" => {
            let addr = "osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63";
            let converted_addr_bytes = decode_and_convert(&addr).unwrap();
            let balance_key =
                create_account_denom_balance_key(converted_addr_bytes, "uosmo").unwrap();

            (
                KVKey {
                    path: BANK_STORE_KEY.to_string(),
                    key: Binary::new(balance_key),
                },
                BANK_QUERY_REGISTRATION_REPLY_ID,
            )
        }
        _ => return Err(ContractError::UnsupportedModule(path).into()),
    };

    let kv_registration_msg = NeutronMsg::RegisterInterchainQuery {
        query_type: QueryType::KV.into(),
        keys: vec![kv_key],
        transactions_filter: String::new(),
        connection_id,
        update_period,
    };

    // fire registration in a submsg to get the registered query id back
    let submsg = SubMsg::reply_on_success(kv_registration_msg, response_code_id);

    Ok(Response::default().add_submessage(submsg))
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    new_config: LibraryConfigUpdate,
) -> Result<(), LibraryError> {
    new_config.update_config(deps)
}

pub fn process_function(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: FunctionMsgs,
    _cfg: Config,
) -> Result<Response<NeutronMsg>, LibraryError> {
    Ok(Response::default())
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

    let associated_query_type = ASSOCIATED_QUERY_IDS.load(deps.storage, query_id)?;
    let query_result_str = match associated_query_type {
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

    QUERY_RESULTS.save(deps.storage, query_id, &query_result_str)?;

    Ok(Response::new().add_attribute("query_result", query_result_str))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        GAMM_QUERY_REGISTRATION_REPLY_ID | BANK_QUERY_REGISTRATION_REPLY_ID => {
            try_associate_registered_query_id(deps, msg)
        }
        _ => Err(ContractError::UnknownReplyId(msg.id).into()),
    }
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

    let val_type = match reply.id {
        GAMM_QUERY_REGISTRATION_REPLY_ID => QueryResult::Gamm {
            result_type: GammResultTypes::Pool,
        },
        BANK_QUERY_REGISTRATION_REPLY_ID => QueryResult::Bank {
            result_type: BankResultTypes::AccountDenomBalance,
        },
        _ => return Err(ContractError::UnknownReplyId(reply.id).into()),
    };
    ASSOCIATED_QUERY_IDS.save(deps.storage, resp.id, &val_type)?;

    Ok(Response::default())
}
