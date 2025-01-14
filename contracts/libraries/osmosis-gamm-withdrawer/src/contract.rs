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
use valence_library_utils::{
    error::LibraryError,
    execute_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};
use valence_osmosis_utils::utils::{
    gamm_utils::ValenceLiquidPooler, get_withdraw_liquidity_msg, DecimalRange,
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            expected_spot_price,
        } => try_withdraw_liquidity(deps, cfg, expected_spot_price),
    }
}

fn try_withdraw_liquidity(
    deps: DepsMut,
    cfg: Config,
    expected_spot_price: Option<DecimalRange>,
) -> Result<Response, LibraryError> {
    let pm_querier = PoolmanagerQuerier::new(&deps.querier);

    // assert the spot price to be within our expectations,
    // if expectations are set.
    if let Some(acceptable_spot_price_range) = expected_spot_price {
        let pool_ratio = pm_querier.query_spot_price(
            cfg.lw_config.pool_id,
            cfg.lw_config.pool_asset_1,
            cfg.lw_config.pool_asset_2,
        )?;

        // perform the spot price validation
        acceptable_spot_price_range.contains(pool_ratio)?;
    }

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
