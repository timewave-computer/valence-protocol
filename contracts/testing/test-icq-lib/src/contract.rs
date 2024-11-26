use crate::icq;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{OpenAckVersion, CATCHALL};
use cosmos_sdk_proto::cosmos::base::abci::v1beta1::TxMsgData;
use cosmos_sdk_proto::prost::Message;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_json_binary, to_json_string, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::KeyDeserialize;
use neutron_sdk::interchain_queries::queries::get_raw_interchain_query_result;
use neutron_sdk::interchain_txs::helpers::decode_message_response;
use neutron_sdk::sudo::msg::RequestPacket;
use neutron_sdk::NeutronError;
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    interchain_queries::v047::queries::{query_balance, BalanceResponse},
    sudo::msg::SudoMsg,
    NeutronResult,
};
use serde_json::value::Serializer;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type QueryDeps<'a> = Deps<'a, NeutronQuery>;
pub type ExecuteDeps<'a> = DepsMut<'a, NeutronQuery>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: ExecuteDeps,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> NeutronResult<Response<NeutronMsg>> {
    match msg {
        ExecuteMsg::RegisterBalancesQuery {
            connection_id,
            update_period,
            addr,
            denoms,
        } => icq::register_balances_query(connection_id, addr, denoms, update_period),
        ExecuteMsg::RegisterKeyValueQuery {
            connection_id,
            update_period,
            path,
            key,
        } => icq::register_kv_query(connection_id, update_period, path, key),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: QueryDeps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { query_id } => to_json_binary(&query_icq_balance(deps, env, query_id)?),
        QueryMsg::Catchall {} => {
            let mut resp: Vec<(String, String)> = vec![];
            for e in CATCHALL.range(deps.storage, None, None, cosmwasm_std::Order::Ascending) {
                resp.push(e?);
            }

            to_json_binary(&resp)
        }
        QueryMsg::RawIcqResult { id } => {
            let resp = get_raw_interchain_query_result(deps, id)
                .map_err(|e| StdError::generic_err(e.to_string()))?;
            to_json_binary(&resp.result)
        }
    }
}

fn query_icq_balance(deps: QueryDeps, env: Env, query_id: u64) -> StdResult<BalanceResponse> {
    query_balance(deps, env, query_id).map_err(|e| StdError::generic_err(e.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: ExecuteDeps, _env: Env, _msg: Reply) -> StdResult<Response<NeutronMsg>> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: ExecuteDeps, _env: Env, _msg: MigrateMsg) -> StdResult<Response<NeutronMsg>> {
    Ok(Response::default())
}

// neutron uses the `sudo` entry point in their ICA/ICQ related logic
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: ExecuteDeps, env: Env, msg: SudoMsg) -> StdResult<Response<NeutronMsg>> {
    match msg {
        // For handling successful registering of ICA
        SudoMsg::OpenAck {
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
        } => sudo_open_ack(
            deps,
            env,
            port_id,
            channel_id,
            counterparty_channel_id,
            counterparty_version,
        ),
        // For handling tx query result
        SudoMsg::TxQueryResult {
            query_id,
            height,
            data,
        } => icq::sudo_tx_query_result(deps, env, query_id, height, data),

        // For handling kv query result
        SudoMsg::KVQueryResult { query_id } => icq::sudo_kv_query_result(deps, env, query_id),
        SudoMsg::Response { request, data } => sudo_response(deps, env, request, data),
        _ => {
            let k = "sudo_catchall_handler".to_string();
            let v = to_json_string(&msg)?;
            CATCHALL.save(deps.storage, k, &v)?;
            Ok(Response::default())
        }
    }
}

fn sudo_response(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    request: RequestPacket,
    data: Binary,
) -> StdResult<Response<NeutronMsg>> {
    let _seq_id = request
        .sequence
        .ok_or_else(|| StdError::generic_err("sequence not found"))?;

    let msg_data: TxMsgData =
        TxMsgData::decode(data.as_slice()).map_err(|e| StdError::generic_err(e.to_string()))?;
    deps.api
        .debug(&format!("WASMDEBUG: msg_data: data: {msg_data:?}"));

    #[allow(deprecated)]
    for item in msg_data.data {
        // let resp = decode_message_response(&item.data);
        // let (k, v) = match serde_json::to_value(&item) {
        //     Ok(val) => ,
        //     Err(e) => todo!(),
        // };
        let k = to_json_string(&request)?;
        // let v = to_json_string(&item)?;
        let v = String::from_vec(item.data.to_ascii_lowercase())?;

        CATCHALL.save(deps.storage, k, &v)?;
    }

    Ok(Response::default())
}

// handler
fn sudo_open_ack(
    deps: ExecuteDeps,
    _env: Env,
    port_id: String,
    _channel_id: String,
    _counterparty_channel_id: String,
    counterparty_version: String,
) -> StdResult<Response<NeutronMsg>> {
    // parse the response
    let parsed_version: OpenAckVersion =
        serde_json_wasm::from_str(counterparty_version.as_str())
            .map_err(|_| StdError::generic_err("Can't parse counterparty_version"))?;

    CATCHALL.save(deps.storage, "sudo_open_ack".to_string(), &port_id)?;

    Ok(Response::default())
}
