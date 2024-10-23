use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, SubMsgResult, Uint128, Uint64,
};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::concentratedliquidity::v1beta1::{
        MsgCreatePosition, MsgCreatePositionResponse, MsgTransferPositions,
    },
};

use valence_account_utils::msg::{parse_valence_payload, ValenceCallback};

use valence_osmosis_utils::utils::cl_utils::{query_cl_pool, TickRange};
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
    msg: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(deps, env, info, msg, process_action, update_config)
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    new_config: ServiceConfigUpdate,
) -> Result<(), ServiceError> {
    new_config.update_config(deps)
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
            tick_range,
            token_min_amount_0,
            token_min_amount_1,
        } => provide_liquidity_custom(
            deps,
            cfg,
            tick_range,
            token_min_amount_0.unwrap_or_default(),
            token_min_amount_1.unwrap_or_default(),
        ),
        ActionMsgs::ProvideLiquidityDefault { bucket_amount } => {
            provide_liquidity_default(deps, cfg, bucket_amount)
        }
    }
}

pub fn provide_liquidity_custom(
    deps: DepsMut,
    cfg: Config,
    tick_range: TickRange,
    token_min_amount_0: Uint128,
    token_min_amount_1: Uint128,
) -> Result<Response, ServiceError> {
    // first we validate the custom target range
    tick_range.validate()?;

    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_1.as_str())?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_2.as_str())?;

    // query the pool config
    let pool_cfg = query_cl_pool(&deps, cfg.lp_config.pool_id.u64())?;

    // the target range must respect the pool tick spacing configuration
    tick_range.ensure_pool_spacing_compatibility(&pool_cfg)?;

    // the target range must be contained within the global tick range
    cfg.lp_config
        .global_tick_range
        .ensure_contains(&tick_range)?;

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id.u64(),
        sender: cfg.input_addr.to_string(),
        lower_tick: tick_range.lower_tick.i64(),
        upper_tick: tick_range.upper_tick.i64(),
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

pub fn provide_liquidity_default(
    deps: DepsMut,
    cfg: Config,
    bucket_count: Uint64,
) -> Result<Response, ServiceError> {
    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_1.as_str())?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, cfg.lp_config.pool_asset_2.as_str())?;

    // query the pool config
    let pool_cfg = query_cl_pool(&deps, cfg.lp_config.pool_id.u64())?;

    // we derive the tick range from the bucket count
    let active_bucket = TickRange::try_from(pool_cfg)?;

    // we extend the active bucket range to both sides by the bucket count
    let derived_tick_range = active_bucket.amplify_range_bidirectionally(bucket_count)?;

    // the target range must be contained within the global tick range
    cfg.lp_config
        .global_tick_range
        .ensure_contains(&derived_tick_range)?;

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id.u64(),
        sender: cfg.input_addr.to_string(),
        lower_tick: derived_tick_range.lower_tick.i64(),
        upper_tick: derived_tick_range.upper_tick.i64(),
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
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ServiceError> {
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
