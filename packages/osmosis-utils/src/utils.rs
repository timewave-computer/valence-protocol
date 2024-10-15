use cosmwasm_std::{Coin, CosmosMsg, StdResult};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::gamm::v1beta1::{MsgJoinPool, MsgJoinSwapExternAmountIn},
};

pub fn get_provide_liquidity_msg(
    input_addr: &str,
    pool_id: u64,
    provision_coins: Vec<Coin>,
    share_out_amt: String,
) -> StdResult<CosmosMsg> {
    let tokens_in_proto = cosmwasm_to_proto_coins(provision_coins);

    let msg_join_pool_no_swap: CosmosMsg = MsgJoinPool {
        sender: input_addr.to_string(),
        pool_id,
        share_out_amount: share_out_amt,
        token_in_maxs: tokens_in_proto,
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

// pub fn get_pool_ratio(pool: Pool, asset_1: String, asset_2: String) -> StdResult<Decimal> {
//     let (mut asset1_balance, mut asset2_balance) = (Uint128::zero(), Uint128::zero());

//     for asset in pool.pool_assets {
//         match asset.token {
//             Some(c) => {
//                 // let cw_coin = try_proto_to_cosmwasm_coins(vec![c])?;
//                 let coin = Coin {
//                     denom: c.denom,
//                     amount: Uint128::from_str(c.amount.as_str())?,
//                 };
//                 if coin.denom == asset_1 {
//                     asset1_balance = coin.amount;
//                 } else if coin.denom == asset_2 {
//                     asset2_balance = coin.amount;
//                 }
//             }
//             None => continue,
//         }
//     }

//     if asset1_balance.is_zero() || asset2_balance.is_zero() {
//         return Err(StdError::generic_err("pool does not contain both assets"));
//     }

//     Ok(Decimal::from_ratio(asset1_balance, asset2_balance))
// }

// pub fn query_pool_asset_balance(deps: &DepsMut, input_addr: &str, asset: &str) -> StdResult<Coin> {
//     let asset_balance = deps.querier.query_balance(input_addr, asset)?;
//     Ok(asset_balance)
// }
