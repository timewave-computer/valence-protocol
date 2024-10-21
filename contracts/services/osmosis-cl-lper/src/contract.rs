use crate::msg::{ActionMsgs, Config, OptionalServiceConfig, QueryMsg, ServiceConfig};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, ensure, to_json_binary, to_json_string, Binary, CosmosMsg, Deps, DepsMut, Env, Int64,
    MessageInfo, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult, Uint128,
};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::{
        concentratedliquidity::v1beta1::{
            MsgCreatePosition, MsgCreatePositionResponse, MsgTransferPositions, Pool,
        },
        poolmanager::v1beta1::PoolmanagerQuerier,
    },
};

use valence_account_utils::msg::{parse_valence_payload, ValenceCallback};
use valence_osmosis_utils::utils::LiquidityProviderConfig;
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
    msg: ExecuteMsg<ActionMsgs, OptionalServiceConfig>,
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
    msg: ActionMsgs,
    cfg: Config,
) -> Result<Response, ServiceError> {
    match msg {
        ActionMsgs::ProvideLiquidityCustom {
            lower_tick,
            upper_tick,
            token_min_amount_0,
            token_min_amount_1,
        } => provide_liquidity_custom(
            deps,
            cfg,
            lower_tick.i64(),
            upper_tick.i64(),
            token_min_amount_0.unwrap_or_default(),
            token_min_amount_1.unwrap_or_default(),
        ),
        ActionMsgs::ProvideLiquidityDefault {} => provide_liquidity_default(deps, cfg),
    }
}

pub fn provide_liquidity_custom(
    deps: DepsMut,
    cfg: Config,
    lower_tick: i64,
    upper_tick: i64,
    token_min_amount_0: Uint128,
    token_min_amount_1: Uint128,
) -> Result<Response, ServiceError> {
    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_1.as_str())?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_2.as_str())?;

    let (global_min, global_max) = (
        cfg.lp_config.global_tick_min.i64(),
        cfg.lp_config.global_tick_max.i64(),
    );

    ensure!(
        lower_tick >= global_min && upper_tick <= global_max && lower_tick < upper_tick,
        StdError::generic_err("tick range validation error")
    );

    // let current_tick =
    //     validate_tick_range(&deps, cfg.lp_config.pool_id.u64(), (global_min, global_max))?;

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id.u64(),
        sender: cfg.input_addr.to_string(),
        lower_tick,
        upper_tick,
        tokens_provided: cosmwasm_to_proto_coins(vec![bal_asset_1, bal_asset_2]),
        token_min_amount0: token_min_amount_0.to_string(),
        token_min_amount1: token_min_amount_1.to_string(),
    }
    .into();

    // we delegate the create position msg as a submsg as we will need to transfer
    // the position afterwards. reply_always so that the saved state is cleared on error.
    let delegated_input_acc_msgs = execute_submsgs_on_behalf_of(
        vec![SubMsg::reply_always(create_cl_position_msg, REPLY_ID)],
        Some(to_json_string(&cfg)?),
        &cfg.input_addr.clone(),
    )?;

    let service_submsg = SubMsg::reply_on_success(delegated_input_acc_msgs, REPLY_ID);

    Ok(Response::default().add_submessage(service_submsg))
}

pub fn provide_liquidity_default(deps: DepsMut, cfg: Config) -> Result<Response, ServiceError> {
    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_1.as_str())?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_2.as_str())?;

    let (global_min, global_max) = (
        cfg.lp_config.global_tick_min.i64(),
        cfg.lp_config.global_tick_max.i64(),
    );

    let current_tick =
        validate_tick_range(&deps, cfg.lp_config.pool_id.u64(), (global_min, global_max))?;

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id.u64(),
        sender: cfg.input_addr.to_string(),
        lower_tick: global_min,
        upper_tick: global_max,
        tokens_provided: cosmwasm_to_proto_coins(vec![bal_asset_1, bal_asset_2]),
        token_min_amount0: "0".to_string(),
        token_min_amount1: "0".to_string(),
    }
    .into();

    // we delegate the position creation message to the input account
    let delegated_input_acc_msgs = execute_submsgs_on_behalf_of(
        // we expect a reply from this submsg so we pass it as a submessage
        vec![SubMsg::reply_always(create_cl_position_msg, REPLY_ID)],
        // associate this msg with a cfg payload which will be used in the reply
        // to restore the state used during this function
        Some(to_json_string(&cfg)?),
        &cfg.input_addr.clone(),
    )?;

    Ok(Response::default()
        .add_submessage(SubMsg::reply_on_success(delegated_input_acc_msgs, REPLY_ID)))
}

fn validate_global_min_max_ranges(
    deps: &DepsMut,
    min: Int64,
    max: Int64,
    lp_config: &LiquidityProviderConfig,
) -> StdResult<()> {
    Ok(())
}

fn validate_tick_range(deps: &DepsMut, pool_id: u64, range: (i64, i64)) -> StdResult<i64> {
    let querier = PoolmanagerQuerier::new(&deps.querier);
    let proto_pool = querier
        .pool(pool_id)?
        .pool
        .ok_or(StdError::generic_err("failed to query pool"))?;

    let pool: Pool = proto_pool
        .try_into()
        .map_err(|_| StdError::generic_err("failed to decode proto pool"))?;

    deps.api.debug(format!("pool config: {:?}", pool).as_str());

    if pool.current_tick >= range.0 && pool.current_tick <= range.1 {
        Ok(pool.current_tick)
    } else {
        Err(StdError::generic_err(
            format!(
                "current tick {} not in range ({}, {})",
                pool.current_tick, range.0, range.1
            )
            .as_str(),
        ))
    }
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
    deps.api.debug(format!("reply: {:?}", msg).as_str());
    match msg.id {
        REPLY_ID => handle_liquidity_provision_reply_id(msg.result),
        _ => Err(ServiceError::Std(StdError::generic_err("unknown reply id"))),
    }
}

fn handle_liquidity_provision_reply_id(result: SubMsgResult) -> Result<Response, ServiceError> {
    // load the config that was used during the initiating message
    // which triggered this reply
    let cfg: Config = parse_valence_payload(&result)?;
    // decode the response from the submsg result
    let valence_callback = ValenceCallback::try_from(result)?;

    // decode the underlying position creation response
    let decoded_resp: MsgCreatePositionResponse = valence_callback.result.try_into()?;

    let transfer_positions_msg = MsgTransferPositions {
        position_ids: vec![decoded_resp.position_id],
        sender: cfg.input_addr.to_string(),
        new_owner: cfg.output_addr.to_string(),
    };

    Ok(Response::default().add_message(execute_on_behalf_of(
        vec![transfer_positions_msg.into()],
        &cfg.input_addr.clone(),
    )?))
}
