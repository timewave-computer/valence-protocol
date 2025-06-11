use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Api, Decimal, Deps};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

use crate::error::LibraryError;

#[cw_serde]
pub struct InstantiateMsg<T> {
    pub owner: String,
    pub processor: String,
    pub config: T,
}

pub trait LibraryConfigValidation<T> {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn Api) -> Result<(), LibraryError>;
    fn validate(&self, deps: Deps) -> Result<T, LibraryError>;
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg<T, U> {
    ProcessFunction(T),
    UpdateConfig { new_config: U },
    UpdateProcessor { processor: String },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum DynamicRatioQueryMsg {
    #[returns(DynamicRatioResponse)]
    DynamicRatio { denoms: Vec<String>, params: String },
}

#[cw_serde]
#[allow(dead_code)]
pub struct DynamicRatioResponse {
    pub denom_ratios: HashMap<String, Decimal>,
}
