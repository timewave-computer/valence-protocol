use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use service_base::{msg::ServiceConfigValidation, ServiceError};
use services_utils::ServiceConfigInterface;
use valence_macros::OptionalStruct;

#[cw_serde]
pub enum ActionsMsgs {
    NoOp {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetOwner {},
    #[returns(Config)]
    GetServiceConfig {},
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {}

impl ServiceConfigValidation<Config> for ServiceConfig {
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
