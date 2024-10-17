use crate::{
    msg::{ActionsMsgs, Config, OptionalServiceConfig, QueryMsg, ServiceConfig},
    state::PENDING_CTX,
};
use base64::decode;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, Int64, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, SubMsgResponse, SubMsgResult, Uint128,
};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::{
        concentratedliquidity::v1beta1::{
            ConcentratedliquidityQuerier, MsgCreatePosition, MsgCreatePositionResponse,
            MsgTransferPositions, Pool,
        },
        poolmanager::v1beta1::PoolmanagerQuerier,
    },
};
use valence_account_utils::msg::ValenceCallback;
use valence_service_utils::{
    error::ServiceError,
    execute_on_behalf_of, execute_submsgs_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPLY_ID: u64 = 314;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionsMsgs, OptionalServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(deps, env, info, msg, process_action, update_config)
}

pub fn update_config(
    deps: &DepsMut,
    _env: Env,
    _info: MessageInfo,
    config: &mut Config,
    new_config: OptionalServiceConfig,
) -> Result<(), ServiceError> {
    new_config.update_config(deps, config)
}

pub fn process_action(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ActionsMsgs,
    cfg: Config,
) -> Result<Response, ServiceError> {
    match msg {
        ActionsMsgs::ProvideDoubleSidedLiquidity {
            lower_tick,
            upper_tick,
            token_min_amount_0,
            token_min_amount_1,
        } => provide_double_sided_liquidity(
            deps,
            cfg,
            lower_tick,
            upper_tick,
            token_min_amount_0,
            token_min_amount_1,
        ),
        ActionsMsgs::ProvideSingleSidedLiquidity {
            asset,
            limit,
            lower_tick,
            upper_tick,
        } => provide_single_sided_liquidity(deps, cfg, asset, limit, lower_tick, upper_tick),
    }
}

pub fn provide_double_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
    lower_tick: Int64,
    upper_tick: Int64,
    token_min_amount_0: Uint128,
    token_min_amount_1: Uint128,
) -> Result<Response, ServiceError> {
    deps.api
        .debug("[OSMO CL LPER] provide double sided liquidity for concentrated liquidity pool");
    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_1.as_str())?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_2.as_str())?;

    let create_cl_position_msg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id,
        sender: cfg.input_addr.to_string(),
        lower_tick: lower_tick.i64(),
        upper_tick: upper_tick.i64(),
        tokens_provided: cosmwasm_to_proto_coins(vec![bal_asset_1, bal_asset_2]),
        token_min_amount0: token_min_amount_0.to_string(),
        token_min_amount1: token_min_amount_1.to_string(),
    };

    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![create_cl_position_msg.into()], &cfg.input_addr.clone())?;

    PENDING_CTX.save(deps.storage, &cfg)?;

    Ok(Response::default()
        .add_submessage(SubMsg::reply_on_success(delegated_input_acc_msgs, REPLY_ID)))
}

fn get_transfer_position_msg(position_ids: Vec<u64>, from: &str, to: &str) -> CosmosMsg {
    MsgTransferPositions {
        position_ids,
        sender: from.to_string(),
        new_owner: to.to_string(),
    }
    .into()
}

fn query_cl_positions(
    deps: &DepsMut,
    pool_id: u64,
    account: String,
) -> Result<Vec<u64>, ServiceError> {
    let pool_manager_querier = ConcentratedliquidityQuerier::new(&deps.querier);
    let active_positions_response = pool_manager_querier.user_positions(account, pool_id, None)?;

    let mut ids: Vec<u64> = vec![];
    for active_position in active_positions_response.positions {
        if let Some(p) = active_position.position {
            ids.push(p.position_id);
        }
    }

    Ok(ids)
}

pub fn provide_single_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
    asset: String,
    limit: Uint128,
    lower_tick: Int64,
    upper_tick: Int64,
) -> Result<Response, ServiceError> {
    // first we assert the input account balance
    let input_acc_asset_bal = deps.querier.query_balance(&cfg.input_addr, &asset)?;

    let provision_amount = if input_acc_asset_bal.amount > limit {
        limit
    } else {
        input_acc_asset_bal.amount
    };

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id,
        sender: cfg.input_addr.to_string(),
        lower_tick: lower_tick.i64(),
        upper_tick: upper_tick.i64(),
        tokens_provided: cosmwasm_to_proto_coins(vec![coin(
            provision_amount.u128(),
            asset.to_string(),
        )]),
        token_min_amount0: "0".to_string(),
        token_min_amount1: "0".to_string(),
    }
    .into();

    let sub_msg = SubMsg::reply_always(create_cl_position_msg, REPLY_ID);
    let delegated_input_acc_msgs =
        execute_submsgs_on_behalf_of(vec![sub_msg], &cfg.input_addr.clone())?;

    PENDING_CTX.save(deps.storage, &cfg)?;

    Ok(Response::default()
        .add_submessage(SubMsg::reply_on_success(delegated_input_acc_msgs, REPLY_ID)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_service_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_service_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = valence_service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ServiceError> {
    deps.api
        .debug(format!("[OSMO CL LPER] reply callback: {:?}", msg.result).as_str());

    match msg.id {
        REPLY_ID => handle_liquidity_provision_reply_id(deps, msg.result),
        _ => Err(ServiceError::Std(StdError::generic_err("unknown reply id"))),
    }
}

fn handle_liquidity_provision_reply_id(
    deps: DepsMut,
    result: SubMsgResult,
) -> Result<Response, ServiceError> {
    let valence_callback = ValenceCallback::try_from_sub_msg_result(result)?;
    deps.api
        .debug(format!("valence callback: {:?}", valence_callback).as_str());

    let decoded_resp: MsgCreatePositionResponse = valence_callback.result.try_into()?;

    deps.api
        .debug(format!("decoded msg create position response: {:?}", decoded_resp).as_str());

    let cfg = PENDING_CTX.load(deps.storage)?;
    let input_acc_positions =
        query_cl_positions(&deps, cfg.lp_config.pool_id, cfg.input_addr.to_string())?;

    let transfer_positions_msg = get_transfer_position_msg(
        input_acc_positions,
        cfg.input_addr.as_str(),
        cfg.output_addr.as_str(),
    );
    deps.api
        .debug("[OSMO CL LPER] about to execute on behalf of input addr from cl lper reply");
    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![transfer_positions_msg], &cfg.input_addr.clone())?;

    Ok(Response::default().add_message(delegated_input_acc_msgs))
}
