use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CosmosMsg, Decimal, DepsMut, StdError, StdResult, Uint128};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::gamm::v1beta1::{GammQuerier, MsgJoinPool, MsgJoinSwapExternAmountIn, Pool},
};

#[cw_serde]
pub enum OsmosisPoolType {
    // gamm, xyk, defined in x/gamm
    Balancer,
    /// cfmm stableswap curve, defined in x/gamm
    StableSwap,
    // CL pool, defined in x/concentrated-liquidity
    Concentrated,
}

pub fn get_provide_liquidity_msg(
    input_addr: &str,
    pool_id: u64,
    provision_coins: Vec<Coin>,
    share_out_amt: String,
) -> StdResult<CosmosMsg> {
    let msg_join_pool_no_swap: CosmosMsg = MsgJoinPool {
        sender: input_addr.to_string(),
        pool_id,
        share_out_amount: share_out_amt,
        token_in_maxs: cosmwasm_to_proto_coins(provision_coins),
    }
    .into();

    Ok(msg_join_pool_no_swap)
}

pub fn get_provide_ss_liquidity_msg(
    input_addr: &str,
    pool_id: u64,
    provision_coin: Coin,
    share_out_amt: String,
) -> StdResult<CosmosMsg> {
    let proto_coin_in = cosmwasm_to_proto_coins(vec![provision_coin]);

    let msg_join_pool_yes_swap: CosmosMsg = MsgJoinSwapExternAmountIn {
        sender: input_addr.to_string(),
        pool_id,
        token_in: Some(proto_coin_in[0].clone()),
        share_out_min_amount: share_out_amt,
    }
    .into();

    Ok(msg_join_pool_yes_swap)
}

pub fn query_pool(deps: &DepsMut, pool_id: u64) -> StdResult<Pool> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    // TODO: switch to the following:
    // let pool_manager = PoolmanagerQuerier::new(&deps.querier);
    // let pool_query_response = pool_manager.pool(pool_id)?;

    let pool_query_response = gamm_querier.pool(pool_id)?;
    let matched_pool: Pool = match pool_query_response.pool {
        Some(any_pool) => any_pool
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode proto"))?,
        None => return Err(StdError::generic_err("pool not found")),
    };
    deps.api
        .debug(&format!("pool response: {:?}", matched_pool));
    Ok(matched_pool)
}

pub fn get_pool_ratio(pool: Pool, asset_1: String, asset_2: String) -> StdResult<Decimal> {
    let (mut asset1_balance, mut asset2_balance) = (Uint128::zero(), Uint128::zero());

    for asset in pool.pool_assets {
        match asset.token {
            Some(c) => {
                // let cw_coin = try_proto_to_cosmwasm_coins(vec![c])?;
                let coin = Coin {
                    denom: c.denom,
                    amount: Uint128::from_str(c.amount.as_str())?,
                };
                if coin.denom == asset_1 {
                    asset1_balance = coin.amount;
                } else if coin.denom == asset_2 {
                    asset2_balance = coin.amount;
                }
            }
            None => continue,
        }
    }

    if asset1_balance.is_zero() || asset2_balance.is_zero() {
        return Err(StdError::generic_err("pool does not contain both assets"));
    }

    Ok(Decimal::from_ratio(asset1_balance, asset2_balance))
}

pub fn query_pool_asset_balance(deps: &DepsMut, input_addr: &str, asset: &str) -> StdResult<Coin> {
    let asset_balance = deps.querier.query_balance(input_addr, asset)?;
    Ok(asset_balance)
}
