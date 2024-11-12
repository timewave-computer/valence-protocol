use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, SubMsgResult, Uint128,
};

use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    MsgWithdrawPosition, MsgWithdrawPositionResponse, PositionByIdRequest, PositionByIdResponse,
};
use valence_account_utils::msg::{parse_valence_payload, ValenceCallback};
use valence_library_utils::{
    error::LibraryError,
    execute_on_behalf_of, execute_submsgs_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};
use valence_osmosis_utils::utils::cl_utils::query_cl_pool;

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPLY_ID: u64 = 314;

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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(deps, env, info, msg, process_action, update_config)
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    new_config: LibraryConfigUpdate,
) -> Result<(), LibraryError> {
    new_config.update_config(deps)
}

pub fn process_action(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
    cfg: Config,
) -> Result<Response, LibraryError> {
    match msg {
        FunctionMsgs::WithdrawLiquidity {
            position_id,
            liquidity_amount,
        } => try_liquidate_cl_position(deps, cfg, position_id.into(), liquidity_amount),
    }
}

pub fn try_liquidate_cl_position(
    deps: DepsMut,
    cfg: Config,
    position_id: u64,
    liquidity_amount: String,
) -> Result<Response, LibraryError> {
    // here we just assert that the position exists.
    // any validations beyond this (like position ownership, etc.)
    // will propagate on execution.
    let _position_query_response: PositionByIdResponse = deps
        .querier
        .query(&PositionByIdRequest { position_id }.into())
        .map_err(|_| StdError::generic_err("no such position"))?;

    let liquidate_position_msg = MsgWithdrawPosition {
        position_id,
        sender: cfg.input_addr.to_string(),
        liquidity_amount,
    };

    // we delegate the position liquidation msg as a submsg because we
    // will need to transfer the underlying tokens we liquidate afterwards.
    let delegated_input_acc_msgs = execute_submsgs_on_behalf_of(
        vec![SubMsg::reply_on_success(liquidate_position_msg, REPLY_ID)],
        Some(to_json_string(&cfg)?),
        &cfg.input_addr.clone(),
    )?;

    let lib_submsg = SubMsg::reply_on_success(delegated_input_acc_msgs, REPLY_ID);

    Ok(Response::default().add_submessage(lib_submsg))
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        REPLY_ID => handle_liquidity_withdrawal_reply(deps.as_ref(), msg.result),
        _ => Err(LibraryError::Std(StdError::generic_err("unknown reply id"))),
    }
}

fn handle_liquidity_withdrawal_reply(
    deps: Deps,
    result: SubMsgResult,
) -> Result<Response, LibraryError> {
    // load the config that was used during the initiating message
    // which triggered this reply
    let cfg: Config = parse_valence_payload(&result)?;

    // decode the response from the submsg result
    let valence_callback = ValenceCallback::try_from(result)?;

    // decode the underlying position withdrawal response
    // and query the pool to match the denoms
    let decoded_resp: MsgWithdrawPositionResponse = valence_callback.result.try_into()?;
    let pool = query_cl_pool(&deps, cfg.pool_id.u64())?;

    let transfer_msg = BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: vec![
            Coin::new(Uint128::from_str(&decoded_resp.amount0)?, pool.token0),
            Coin::new(Uint128::from_str(&decoded_resp.amount1)?, pool.token1),
        ],
    };

    Ok(Response::default().add_message(execute_on_behalf_of(
        vec![transfer_msg.into()],
        &cfg.input_addr.clone(),
    )?))
}
