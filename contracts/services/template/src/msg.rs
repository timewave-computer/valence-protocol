use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_macros::{valence_service_query, OptionalStruct};
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceConfigInterface,
};

#[cw_serde]
pub enum ActionsMsgs {
    NoOp {},
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, _api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        Ok(())
    }

    fn validate(&self, _deps: Deps) -> Result<Config, ServiceError> {
        Ok(Config {})
    }
}

impl ServiceConfigInterface<ServiceConfig> for ServiceConfig {
    /// This function is used to see if 2 configs are different
    fn is_diff(&self, other: &ServiceConfig) -> bool {
        !self.eq(other)
    }
}

impl OptionalServiceConfig {
    pub fn update_config(self, _deps: &DepsMut, _config: &mut Config) -> Result<(), ServiceError> {
        Ok(())
    }
}

#[cw_serde]
pub struct Config {}
