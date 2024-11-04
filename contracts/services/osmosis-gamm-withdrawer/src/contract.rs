use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, coins, to_json_binary, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env, Fraction,
    MessageInfo, Response, StdError, StdResult, Uint128,
};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::{gamm::v1beta1::GammQuerier, poolmanager::v1beta1::PoolmanagerQuerier},
};
use valence_osmosis_utils::utils::{get_provide_ss_liquidity_msg, get_withdraw_liquidity_msg};
use valence_service_utils::{
    error::ServiceError,
    execute_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{
    ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate, ValenceLiquidPooler,
};

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
    // first we assert the input account balance
    // let input_acc_asset_bal = deps.querier.query_balance(&cfg.input_addr, &asset)?;
    let pm_querier = PoolmanagerQuerier::new(&deps.querier);
    let pool = pm_querier.query_pool_config(cfg.lp_config.pool_id)?;

    let liquidity_provision_msg = get_withdraw_liquidity_msg()?;

    let transfer_lp_tokens_msg = BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: coins(
            share_out_amt.u128(),
            pool.total_shares
                .ok_or_else(|| StdError::generic_err("failed to get shares"))?
                .denom,
        ),
    };

    let delegated_input_acc_msgs = execute_on_behalf_of(
        vec![liquidity_provision_msg, transfer_lp_tokens_msg.into()],
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

pub fn calculate_share_out_amt_no_swap(
    deps: &DepsMut,
    pool_id: u64,
    coins_in: Vec<Coin>,
) -> StdResult<Uint128> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    let shares_out = gamm_querier
        .calc_join_pool_no_swap_shares(pool_id, cosmwasm_to_proto_coins(coins_in))?
        .shares_out;

    let shares_u128 = Uint128::from_str(&shares_out)?;

    Ok(shares_u128)
}

pub fn calculate_share_out_amt_swap(
    deps: &DepsMut,
    pool_id: u64,
    coin_in: Vec<Coin>,
) -> StdResult<Uint128> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    let shares_out = gamm_querier
        .calc_join_pool_shares(pool_id, cosmwasm_to_proto_coins(coin_in))?
        .share_out_amount;

    let shares_u128 = Uint128::from_str(&shares_out)?;

    Ok(shares_u128)
}

pub fn calculate_provision_amounts(
    mut asset_1_bal: Coin,
    mut asset_2_bal: Coin,
    pool_ratio: Decimal,
) -> StdResult<Vec<Coin>> {
    // first we assume that we are going to provide all of asset_1 and up to all of asset_2
    // then we get the expected amount of asset_2 tokens to provide
    let expected_asset_2_provision_amt = asset_1_bal
        .amount
        .checked_multiply_ratio(pool_ratio.numerator(), pool_ratio.denominator())
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // then we check if the expected amount of asset_2 tokens is greater than the available balance
    if expected_asset_2_provision_amt > asset_2_bal.amount {
        // if it is, we calculate the amount of asset_1 tokens to provide
        asset_1_bal.amount = asset_2_bal
            .amount
            .checked_multiply_ratio(pool_ratio.denominator(), pool_ratio.numerator())
            .map_err(|e| StdError::generic_err(e.to_string()))?;
    } else {
        // if it is not, we provide all of asset_1 and the expected amount of asset_2
        asset_2_bal.amount = expected_asset_2_provision_amt;
    }

    Ok(vec![asset_1_bal, asset_2_bal])
}
