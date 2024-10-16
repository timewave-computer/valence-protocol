#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, Int64, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::{
        concentratedliquidity::v1beta1::{MsgCreatePosition, Pool},
        poolmanager::v1beta1::PoolmanagerQuerier,
    },
};
use valence_service_utils::{
    error::ServiceError,
    execute_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionsMsgs, Config, OptionalServiceConfig, QueryMsg, ServiceConfig};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        } => provide_double_sided_liquidity(deps, cfg, lower_tick, upper_tick),
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
) -> Result<Response, ServiceError> {
    deps.api
        .debug("provide double sided liquidity for concentrated liquidity pool");
    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_1.as_str())?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_2.as_str())?;

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id,
        sender: cfg.input_addr.to_string(),
        lower_tick: lower_tick.i64(),
        upper_tick: upper_tick.i64(),
        tokens_provided: cosmwasm_to_proto_coins(vec![bal_asset_1, bal_asset_2]),
        // should we be strict here and set them to the actual token amounts?
        token_min_amount0: "0".to_string(),
        token_min_amount1: "0".to_string(),
    }
    .into();

    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![create_cl_position_msg], &cfg.input_addr.clone())?;

    Ok(Response::default().add_message(delegated_input_acc_msgs))
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

    let matched_pool = query_cl_pool(&deps, cfg.lp_config.pool_id)?;

    let (_token_min_amount0, _token_min_amount1) = if matched_pool.token0 == asset {
        ("1".to_string(), "0".to_string())
    } else {
        ("0".to_string(), "1".to_string())
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

    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![create_cl_position_msg], &cfg.input_addr.clone())?;

    Ok(Response::default().add_message(delegated_input_acc_msgs))
}

fn query_cl_pool(deps: &DepsMut, pool_id: u64) -> StdResult<Pool> {
    let querier = PoolmanagerQuerier::new(&deps.querier);
    let pool_query_response = querier.pool(pool_id)?;

    let matched_pool: Pool = match pool_query_response.pool {
        Some(any_pool) => any_pool
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode proto into CL pool type"))?,
        None => return Err(StdError::generic_err("pool not found")),
    };

    deps.api
        .debug(format!("CL pool via poolmanager query: {:?}", matched_pool).as_str());

    Ok(matched_pool)
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
