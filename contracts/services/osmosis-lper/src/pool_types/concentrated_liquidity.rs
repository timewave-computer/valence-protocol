use cosmwasm_std::{coin, CosmosMsg, DepsMut, Response, StdError, StdResult, Uint128};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::{
        concentratedliquidity::v1beta1::{MsgCreatePosition, Pool},
        poolmanager::v1beta1::PoolmanagerQuerier,
    },
};
use valence_osmosis_utils::utils::query_pool_asset_balance;
use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

use crate::valence_service_integration::Config;

pub fn provide_double_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
) -> Result<Response, ServiceError> {
    deps.api
        .debug("provide double sided liquidity for concentrated liquidity pool");
    // first we assert the input account balances
    let bal_asset_1 = query_pool_asset_balance(
        &deps,
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_asset_1.as_str(),
    )?;
    let bal_asset_2 = query_pool_asset_balance(
        &deps,
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_asset_2.as_str(),
    )?;

    deps.api
        .debug(format!("input account pool asset 1 balance: {:?}", bal_asset_1).as_str());
    deps.api
        .debug(format!("input account pool asset 2 balance: {:?}", bal_asset_2).as_str());

    // let matched_pool = query_cl_pool(&deps, cfg.lp_config.pool_id)?;

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id,
        sender: cfg.input_addr.to_string(),
        lower_tick: -10_000,
        upper_tick: 10_000,
        tokens_provided: cosmwasm_to_proto_coins(vec![bal_asset_1, bal_asset_2]),
        // should we be strict here and set them to the actual token amounts?
        token_min_amount0: "0".to_string(),
        token_min_amount1: "0".to_string(),
    }
    .into();

    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![create_cl_position_msg], &cfg.input_addr.clone())?;
    deps.api.debug(
        format!(
            "delegated cl position creation msg: {:?}",
            delegated_input_acc_msgs
        )
        .as_str(),
    );

    Ok(Response::default().add_message(delegated_input_acc_msgs))
}

pub fn provide_single_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
    asset: String,
    limit: Uint128,
) -> Result<Response, ServiceError> {
    // first we assert the input account balance
    let input_acc_asset_bal = query_pool_asset_balance(&deps, cfg.input_addr.as_str(), &asset)?;

    deps.api.debug(
        format!(
            "input account pool asset balance: {:?}",
            input_acc_asset_bal
        )
        .as_str(),
    );

    let provision_amount = if input_acc_asset_bal.amount > limit {
        limit
    } else {
        input_acc_asset_bal.amount
    };

    let matched_pool = query_cl_pool(&deps, cfg.lp_config.pool_id)?;

    let (token_min_amount0, token_min_amount1) = if matched_pool.token0 == asset {
        ("1".to_string(), "0".to_string())
    } else {
        ("0".to_string(), "1".to_string())
    };

    let create_cl_position_msg: CosmosMsg = MsgCreatePosition {
        pool_id: cfg.lp_config.pool_id,
        sender: cfg.input_addr.to_string(),
        lower_tick: -1_000,
        upper_tick: 1_000,
        tokens_provided: cosmwasm_to_proto_coins(vec![coin(
            provision_amount.u128(),
            asset.to_string(),
        )]),
        token_min_amount0: "0".to_string(),
        token_min_amount1: "0".to_string(),
    }
    .into();
    deps.api
        .debug(format!("cl position creation msg: {:?}", create_cl_position_msg).as_str());
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
