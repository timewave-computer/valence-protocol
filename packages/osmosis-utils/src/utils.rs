use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Coin, CosmosMsg, Decimal, StdResult, Uint128};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::gamm::v1beta1::{MsgExitPool, MsgJoinPool, MsgJoinSwapExternAmountIn},
};
use valence_service_utils::error::ServiceError;

#[cw_serde]
pub struct LiquidityProviderConfig {
    pub pool_id: u64,
    pub pool_asset_1: String,
    pub pool_asset_2: String,
}

#[cw_serde]
pub struct DecimalRange {
    min: Decimal,
    max: Decimal,
}

impl From<(Decimal, Decimal)> for DecimalRange {
    fn from((min, max): (Decimal, Decimal)) -> Self {
        DecimalRange { min, max }
    }
}

impl DecimalRange {
    pub fn contains(&self, value: Decimal) -> Result<(), ServiceError> {
        ensure!(
            value >= self.min && value <= self.max,
            ServiceError::ExecutionError("Value is not within the expected range".to_string())
        );
        Ok(())
    }
}

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

pub fn get_withdraw_liquidity_msg(
    input_addr: &str,
    pool_id: u64,
    share_in_amount: Uint128,
    token_out_mins: Vec<osmosis_std::types::cosmos::base::v1beta1::Coin>,
) -> StdResult<CosmosMsg> {
    let exit_pool_request: CosmosMsg = MsgExitPool {
        sender: input_addr.to_string(),
        pool_id,
        share_in_amount: share_in_amount.to_string(),
        token_out_mins,
    }
    .into();

    Ok(exit_pool_request)
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

pub mod cl_utils {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{ensure, Deps, Int64, StdError, StdResult, Uint64};
    use osmosis_std::types::osmosis::{
        concentratedliquidity::v1beta1::Pool, poolmanager::v1beta1::PoolmanagerQuerier,
    };
    use valence_service_utils::error::ServiceError;

    pub fn query_cl_pool(deps: &Deps, pool_id: u64) -> StdResult<Pool> {
        let querier = PoolmanagerQuerier::new(&deps.querier);
        let proto_pool = querier
            .pool(pool_id)?
            .pool
            .ok_or(StdError::generic_err("failed to query pool"))?;

        let pool: Pool = proto_pool
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode proto pool"))?;

        Ok(pool)
    }

    #[cw_serde]
    pub struct TickRange {
        pub lower_tick: Int64,
        pub upper_tick: Int64,
    }

    impl TryFrom<Pool> for TickRange {
        type Error = StdError;

        /// method to derive the currently active CL pool bucket from the pool config.
        fn try_from(value: Pool) -> Result<Self, Self::Error> {
            let current_tick = Int64::from(value.current_tick);
            let tick_spacing: Int64 = i64::try_from(value.tick_spacing)
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .into();

            // calculating the lower bound of the current tick range works as follows:
            // 1. Divide the current tick by the tick spacing (using euclidean division).
            //  Euclidian division is used here to cover cases where the current tick is
            //  negative. Regular integer division rounds towards 0, which would return an
            //  off-by-one bucket (towards positive values).
            // 2. Multiply the result by the tick spacing
            let lower_bound = current_tick
                .checked_div_euclid(tick_spacing)
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .checked_mul(tick_spacing)?;

            Ok(TickRange {
                lower_tick: lower_bound,
                upper_tick: lower_bound.checked_add(tick_spacing)?,
            })
        }
    }

    impl TickRange {
        pub fn validate(&self) -> Result<(), ServiceError> {
            ensure!(
                self.lower_tick < self.upper_tick,
                ServiceError::ExecutionError("lower tick must be less than upper tick".to_string())
            );
            Ok(())
        }

        pub fn ensure_pool_spacing_compatibility(&self, pool: &Pool) -> Result<(), ServiceError> {
            let spacing_i64 = i64::try_from(pool.tick_spacing)
                .map_err(|_| ServiceError::Std(StdError::generic_err("failed to cast")))?;

            let lower_compatible = if self.lower_tick.is_zero() {
                // 0 is always considered compatible
                true
            } else {
                self.lower_tick.i64() % spacing_i64 == 0
            };

            let upper_compatible = if self.upper_tick.is_zero() {
                // 0 is always considered compatible
                true
            } else {
                self.upper_tick.i64() % spacing_i64 == 0
            };

            ensure!(
                lower_compatible && upper_compatible,
                ServiceError::ExecutionError(
                    "tick range is not a multiple of the other".to_string()
                )
            );
            Ok(())
        }

        pub fn ensure_contains(&self, other: &TickRange) -> Result<(), ServiceError> {
            ensure!(
                self.lower_tick <= other.lower_tick && self.upper_tick >= other.upper_tick,
                ServiceError::ExecutionError(
                    "other tick range is not contained by this range".to_string()
                )
            );
            Ok(())
        }

        // takes the current bucket and extends its range by wrapping the current
        // range between `multiple` amount of mirrored buckets placed on both sides
        // e.g. for a range of (100, 200) and a multiple of 2, it would obtain the
        // final range by taking [-100, 0], [0, 100], [100, 200], [200, 300], [300, 400]
        // which results in the final range of [-100, 400].
        pub fn amplify_range_bidirectionally(&self, multiple: Uint64) -> StdResult<TickRange> {
            ensure!(
                !multiple.is_zero(),
                StdError::generic_err("cannot have zero multiple")
            );

            let multiple_scaled = Int64::try_from(multiple)?;

            let distance = self.upper_tick.checked_sub(self.lower_tick)?;
            let extension = distance.checked_mul(multiple_scaled)?;

            Ok(TickRange {
                lower_tick: self.lower_tick.checked_sub(extension)?,
                upper_tick: self.upper_tick.checked_add(extension)?,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use cosmwasm_std::{Int64, Uint64};

        fn default_pool() -> Pool {
            Pool {
                current_tick: 100,
                tick_spacing: 10,
                ..Default::default()
            }
        }

        #[test]
        fn test_tick_range_from_pool() {
            let pool = default_pool();
            let tick_range = TickRange::try_from(pool).unwrap();
            assert_eq!(tick_range.lower_tick, Int64::new(100));
            assert_eq!(tick_range.upper_tick, Int64::new(110));
        }

        #[test]
        fn test_tick_range_from_pool_negative_current_tick() {
            let pool = Pool {
                current_tick: -100,
                tick_spacing: 10,
                ..Default::default()
            };
            let tick_range = TickRange::try_from(pool).unwrap();
            assert_eq!(tick_range.lower_tick, Int64::new(-100));
            assert_eq!(tick_range.upper_tick, Int64::new(-90));
        }

        #[test]
        fn test_tick_range_from_pool_positive_current_tick_mid_bucket() {
            let pool = Pool {
                current_tick: 105,
                tick_spacing: 10,
                ..Default::default()
            };
            let tick_range = TickRange::try_from(pool).unwrap();

            assert_eq!(tick_range.lower_tick, Int64::new(100));
            assert_eq!(tick_range.upper_tick, Int64::new(110));
        }

        #[test]
        fn test_tick_range_from_pool_negative_current_tick_mid_bucket() {
            let pool = Pool {
                current_tick: -105,
                tick_spacing: 10,
                ..Default::default()
            };
            let tick_range = TickRange::try_from(pool).unwrap();

            assert_eq!(tick_range.lower_tick, Int64::new(-110));
            assert_eq!(tick_range.upper_tick, Int64::new(-100));
        }

        #[test]
        fn test_tick_range_validate_happy() {
            let valid_range = TickRange {
                lower_tick: Int64::new(100),
                upper_tick: Int64::new(200),
            };
            assert!(valid_range.validate().is_ok());
        }

        #[test]
        #[should_panic(expected = "lower tick must be less than upper tick")]
        fn test_tick_range_validation_panics() {
            let invalid_range = TickRange {
                lower_tick: Int64::new(200),
                upper_tick: Int64::new(100),
            };
            invalid_range.validate().unwrap();
        }

        #[test]
        fn test_tick_range_ensure_multiple_of_happy() {
            let range1 = TickRange {
                lower_tick: Int64::new(100),
                upper_tick: Int64::new(200),
            };

            assert!(range1
                .ensure_pool_spacing_compatibility(&default_pool())
                .is_ok());
        }

        #[test]
        #[should_panic(expected = "tick range is not a multiple of the other")]
        fn test_tick_range_ensure_compatibility_errors() {
            let range1 = TickRange {
                lower_tick: Int64::new(125),
                upper_tick: Int64::new(200),
            };

            range1
                .ensure_pool_spacing_compatibility(&default_pool())
                .unwrap();
        }

        #[test]
        fn test_tick_range_ensure_contains() {
            let outer_range = TickRange {
                lower_tick: Int64::new(0),
                upper_tick: Int64::new(300),
            };
            let inner_range = TickRange {
                lower_tick: Int64::new(100),
                upper_tick: Int64::new(200),
            };
            assert!(outer_range.ensure_contains(&inner_range).is_ok());
            assert!(inner_range.ensure_contains(&outer_range).is_err());
        }

        #[test]
        fn test_tick_range_multiply_range() {
            let range = TickRange {
                lower_tick: Int64::new(100),
                upper_tick: Int64::new(200),
            };
            let multiplied_range = range.amplify_range_bidirectionally(Uint64::new(2)).unwrap();
            assert_eq!(multiplied_range.lower_tick, Int64::new(-100));
            assert_eq!(multiplied_range.upper_tick, Int64::new(400));
        }
    }
}

pub mod gamm_utils {
    use std::str::FromStr;

    use cosmwasm_std::{Decimal, Empty, StdError, StdResult};
    use osmosis_std::types::osmosis::{
        gamm::v1beta1::Pool, poolmanager::v1beta1::PoolmanagerQuerier,
    };

    pub trait ValenceLiquidPooler {
        fn query_spot_price(
            &self,
            pool_id: u64,
            pool_asset_1: String,
            pool_asset_2: String,
        ) -> StdResult<Decimal>;
        fn query_pool_config(&self, pool_id: u64) -> StdResult<Pool>;
        fn query_pool_liquidity_token(&self, pool_id: u64) -> StdResult<String>;
    }

    impl ValenceLiquidPooler for PoolmanagerQuerier<'_, Empty> {
        fn query_spot_price(
            &self,
            pool_id: u64,
            pool_asset_1: String,
            pool_asset_2: String,
        ) -> StdResult<Decimal> {
            let spot_price_response =
                self.spot_price(pool_id, pool_asset_1.to_string(), pool_asset_2.to_string())?;

            let pool_ratio = Decimal::from_str(&spot_price_response.spot_price)?;

            Ok(pool_ratio)
        }

        fn query_pool_config(&self, pool_id: u64) -> StdResult<Pool> {
            let pool_response = self.pool(pool_id)?;
            let pool: Pool = pool_response
                .pool
                .ok_or_else(|| StdError::generic_err("failed to get pool"))?
                .try_into()
                .map_err(|_| StdError::generic_err("failed to decode proto"))?;

            Ok(pool)
        }

        fn query_pool_liquidity_token(&self, pool_id: u64) -> StdResult<String> {
            let pool = self.query_pool_config(pool_id)?;

            match pool.total_shares {
                Some(c) => Ok(c.denom),
                None => Err(StdError::generic_err(
                    "failed to get LP token of given pool",
                )),
            }
        }
    }
}
