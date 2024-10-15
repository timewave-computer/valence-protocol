use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw_ownable::cw_ownable_query;
use valence_osmosis_utils::utils::DecimalRange;

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
