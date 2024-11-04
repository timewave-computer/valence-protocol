#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};
use osmosis_std::{
    try_proto_to_cosmwasm_coins,
    types::osmosis::{
        gamm::v1beta1::{
            QueryCalcExitPoolCoinsFromSharesRequest, QueryCalcExitPoolCoinsFromSharesResponse,
        },
        poolmanager::v1beta1::PoolmanagerQuerier,
    },
};
use valence_osmosis_utils::utils::{gamm_utils::ValenceLiquidPooler, get_withdraw_liquidity_msg};
use valence_service_utils::{
    error::ServiceError,
    execute_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate};

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
        ActionMsgs::WithdrawLiquidity {} => try_withdraw_liquidity(deps, cfg),
    }
}

fn try_withdraw_liquidity(deps: DepsMut, cfg: Config) -> Result<Response, ServiceError> {
    let pm_querier = PoolmanagerQuerier::new(&deps.querier);

    // get the LP token balance of configured input account
    let lp_token = pm_querier.query_pool_liquidity_token(cfg.lw_config.pool_id)?;
    let input_acc_lp_token_bal = deps
        .querier
        .query_balance(&cfg.input_addr, lp_token)?
        .amount;

    // liquidity can be withdrawn iff lp token balance is gt zero
    ensure!(
        input_acc_lp_token_bal > Uint128::zero(),
        StdError::generic_err("input account must have LP tokens to withdraw")
    );

    // simulate the withdrawal to get the expected coins out
    let calc_exit_query_response: QueryCalcExitPoolCoinsFromSharesResponse = deps.querier.query(
        &QueryCalcExitPoolCoinsFromSharesRequest {
            pool_id: cfg.lw_config.pool_id,
            share_in_amount: input_acc_lp_token_bal.to_string(),
        }
        .into(),
    )?;

    // get the liquidity withdrawal message
    let remove_liquidity_msg = get_withdraw_liquidity_msg(
        cfg.input_addr.as_str(),
        cfg.lw_config.pool_id,
        input_acc_lp_token_bal,
        calc_exit_query_response.tokens_out.clone(),
    )?;

    // get the transfer message for underlying assets withdrawn
    let transfer_underlying_coins_msg = BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: try_proto_to_cosmwasm_coins(calc_exit_query_response.tokens_out)?,
    };

    let delegated_input_acc_msgs = execute_on_behalf_of(
        vec![remove_liquidity_msg, transfer_underlying_coins_msg.into()],
        &cfg.input_addr.clone(),
    )?;

    Ok(Response::default().add_message(delegated_input_acc_msgs))
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
        QueryMsg::GetRawServiceConfig {} => {
            let raw_config: ServiceConfig =
                valence_service_utils::raw_config::query_raw_service_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
