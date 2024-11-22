use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, to_json_string, BankMsg, Binary, Coin, CosmosMsg, Decimal256, Deps,
    DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult, Uint128,
};

use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    ConcentratedliquidityQuerier, MsgWithdrawPosition, MsgWithdrawPositionResponse,
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
    valence_library_base::execute(deps, env, info, msg, process_function, update_config)
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
    liquidity_amount: Option<Decimal256>,
) -> Result<Response, LibraryError> {
    // first we query the position
    let position_query_response =
        ConcentratedliquidityQuerier::new(&deps.querier).position_by_id(position_id)?;
    let position = position_query_response
        .position
        .and_then(|pos| pos.position)
        .ok_or_else(|| StdError::generic_err("failed to get cl position"))?;

    // convert the string-based liquidity field to Decimal256
    let total_position_liquidity = Decimal256::from_str(&position.liquidity)?;

    let liquidity_to_withdraw = match liquidity_amount {
        // if liquidity amount to be liquidated is specified,
        // we ensure that the amount is less than or equal to
        // the total liquidity in the position
        Some(amt) => {
            ensure!(
                amt <= total_position_liquidity,
                StdError::generic_err(format!(
                    "Insufficient liquidity: {amt} > {total_position_liquidity}",
                ))
            );
            amt
        }
        // if no liquidity amount is specified, we withdraw the entire position
        None => total_position_liquidity,
    };

    let liquidate_position_msg: CosmosMsg = MsgWithdrawPosition {
        position_id,
        sender: cfg.input_addr.to_string(),
        liquidity_amount: liquidity_to_withdraw.atomics().to_string(),
    }
    .into();

    // we delegate the position liquidation msg as a submsg because we
    // will need to transfer the underlying tokens we liquidate afterwards.
    let delegated_input_acc_msgs = execute_submsgs_on_behalf_of(
        vec![SubMsg::reply_on_success(liquidate_position_msg, REPLY_ID)],
        Some(to_json_string(&cfg)?),
        &cfg.input_addr,
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
    // and query the pool to ensure denom ordering
    let decoded_resp: MsgWithdrawPositionResponse = valence_callback.result.try_into()?;
    let pool = query_cl_pool(&deps, cfg.pool_id.u64())?;

    let mut transfer_coins = vec![];

    let amt_0 = Uint128::from_str(&decoded_resp.amount0)?;
    let amt_1 = Uint128::from_str(&decoded_resp.amount1)?;

    // there may be situations where only one coin was withdrawn.
    // to avoid sending empty coins, we only include non-0-bal coins
    if !amt_0.is_zero() {
        transfer_coins.push(Coin::new(amt_0, pool.token0));
    }
    if !amt_1.is_zero() {
        transfer_coins.push(Coin::new(amt_1, pool.token1));
    }

    // both coins cannot be zero because that would mean the position
    // had no underlying liquidity to withdraw, so we skip the empty
    // array check here and just fire the banksend
    let transfer_msg = BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: transfer_coins,
    };

    Ok(Response::default().add_message(execute_on_behalf_of(
        vec![transfer_msg.into()],
        &cfg.input_addr.clone(),
    )?))
}
