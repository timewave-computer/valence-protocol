use osmosis_std::types::osmosis::concentratedliquidity::poolmodel::concentrated::v1beta1::{
    MsgCreateConcentratedPool, MsgCreateConcentratedPoolResponse,
};
use osmosis_std::types::osmosis::concentratedliquidity::v1beta1::{
    ClaimableSpreadRewardsRequest, ClaimableSpreadRewardsResponse, LiquidityNetInDirectionRequest,
    LiquidityNetInDirectionResponse, LiquidityPerTickRangeRequest, LiquidityPerTickRangeResponse,
    MsgCollectIncentives, MsgCollectIncentivesResponse, MsgCollectSpreadRewards,
    MsgCollectSpreadRewardsResponse, MsgCreatePosition, MsgCreatePositionResponse,
    MsgTransferPositions, MsgTransferPositionsResponse, MsgWithdrawPosition,
    MsgWithdrawPositionResponse, ParamsRequest, ParamsResponse, PoolsRequest, PoolsResponse,
    PositionByIdRequest, PositionByIdResponse, UserPositionsRequest, UserPositionsResponse,
};
use osmosis_test_tube::{fn_execute, fn_query, Module, Runner};

#[cfg(test)]
mod test_suite;
#[cfg(test)]
mod tests;

pub struct ConcentratedLiquidityExt<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for ConcentratedLiquidityExt<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> ConcentratedLiquidityExt<'a, R>
where
    R: Runner<'a>,
{
    // ========== Messages ==========

    // create concentrated pool
    fn_execute! { pub create_concentrated_pool: MsgCreateConcentratedPool => MsgCreateConcentratedPoolResponse }

    // create position
    fn_execute! { pub create_position: MsgCreatePosition => MsgCreatePositionResponse }

    // withdraw position
    fn_execute! { pub withdraw_position: MsgWithdrawPosition => MsgWithdrawPositionResponse }

    // collect spread rewards
    fn_execute! { pub collected_spread_rewards: MsgCollectSpreadRewards => MsgCollectSpreadRewardsResponse }

    // collect incentives
    fn_execute! { pub collect_incentives: MsgCollectIncentives => MsgCollectIncentivesResponse }

    // transfer CL position
    fn_execute! { pub transfer_positions: MsgTransferPositions => MsgTransferPositionsResponse }

    // ========== Queries ==========

    // query pools
    fn_query! {
        pub query_pools ["/osmosis.concentratedliquidity.v1beta1.Query/Pools"]: PoolsRequest => PoolsResponse
    }

    // query params
    fn_query! {
        pub query_params ["/osmosis.concentratedliquidity.v1beta1.Query/Params"]: ParamsRequest => ParamsResponse
    }

    // query liquidity_net_in_direction
    fn_query! {
        pub query_liquidity_depths_for_range ["/osmosis.concentratedliquidity.v1beta1.Query/LiquidityNetInDirection"]: LiquidityNetInDirectionRequest => LiquidityNetInDirectionResponse
    }

    // query user_positions
    fn_query! {
        pub query_user_positions ["/osmosis.concentratedliquidity.v1beta1.Query/UserPositions"]: UserPositionsRequest => UserPositionsResponse
    }

    // query liquidity_net_in_direction
    fn_query! {
        pub query_liquidity_net_in_direction ["/osmosis.concentratedliquidity.v1beta1.Query/LiquidityNetInDirection"]: LiquidityNetInDirectionRequest => LiquidityNetInDirectionResponse
    }

    // query liquidity_per_tick_range
    fn_query! {
        pub query_liquidity_per_tick_range ["/osmosis.concentratedliquidity.v1beta1.Query/LiquidityPerTickRange"]: LiquidityPerTickRangeRequest => LiquidityPerTickRangeResponse
    }

    // query claimable_fees
    fn_query! {
        pub query_claimable_fees ["/osmosis.concentratedliquidity.v1beta1.Query/ClaimableSpreadRewards"]: ClaimableSpreadRewardsRequest => ClaimableSpreadRewardsResponse
    }

    // query position_by_id
    fn_query! {
        pub query_position_by_id ["/osmosis.concentratedliquidity.v1beta1.Query/PositionById"]: PositionByIdRequest => PositionByIdResponse
    }
}
