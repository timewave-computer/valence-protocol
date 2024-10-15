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
