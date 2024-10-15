use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Decimal, Uint128};
use cw_ownable::cw_ownable_query;
use valence_service_utils::error::ServiceError;

#[cw_serde]
pub enum ActionsMsgs {
    ProvideDoubleSidedLiquidity {
        expected_spot_price: Option<DecimalRange>,
    },
    ProvideSingleSidedLiquidity {
        expected_spot_price: Option<DecimalRange>,
        asset: String,
        limit: Uint128,
    },
}

#[cw_serde]
pub struct DecimalRange {
    min: Decimal,
    max: Decimal,
}

impl DecimalRange {
    pub fn is_within_range(&self, value: Decimal) -> Result<(), ServiceError> {
        ensure!(
            value >= self.min && value <= self.max,
            ServiceError::ExecutionError("Value is not within the expected range".to_string())
        );
        Ok(())
    }
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetProcessor {},
    #[returns(crate::valence_service_integration::Config)]
    GetServiceConfig {},
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    pub pool_id: u64,
    pub pool_asset_1: String,
    pub pool_asset_2: String,
}
