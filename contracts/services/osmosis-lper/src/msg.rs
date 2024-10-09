use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps};
use cw_ownable::cw_ownable_query;
use valence_service_utils::error::ServiceError;

use crate::valence_service_integration::Config;

#[cw_serde]
pub enum ActionsMsgs {
    ProvideDoubleSidedLiquidity {},
    ProvideSingleSidedLiquidity {},
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetProcessor {},
    #[returns(Config)]
    GetServiceConfig {},
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    pub pool_id: u64,
}

pub fn ensure_correct_pool(_pool_id: String, _deps: &Deps) -> Result<(), ServiceError> {
    Ok(())
}
