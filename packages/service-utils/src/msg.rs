use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Api, Deps};
use cw_ownable::cw_ownable_execute;

use crate::error::ServiceError;

#[cw_serde]
pub struct InstantiateMsg<T> {
    pub owner: String,
    pub processor: String,
    pub config: T,
}

pub trait ServiceConfigValidation<T> {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn Api) -> Result<(), ServiceError>;
    fn validate(&self, deps: Deps) -> Result<T, ServiceError>;
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg<T, U> {
    ProcessAction(T),
    UpdateConfig { new_config: U },
    UpdateProcessor { processor: String },
}
