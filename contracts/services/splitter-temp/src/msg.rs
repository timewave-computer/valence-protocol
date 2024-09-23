use std::collections::{BTreeMap, BTreeSet};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use valence_macros::OptionalStruct;
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType, ServiceConfigInterface,
};

#[cw_serde]
pub enum ActionsMsgs {
    Split {},
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
pub struct ServiceConfig {
    /// Address we pull funds from
    pub input_addr: ServiceAccountType,
    pub splits: SplitsConfig,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        // TODO: Verify splits are valid
        Ok(Config {
            input_addr: self.input_addr.to_addr(deps)?,
            splits: self.splits.clone(),
        })
    }
}

impl ServiceConfigInterface<ServiceConfig> for ServiceConfig {
    /// This function is used to see if 2 configs are different
    fn is_diff(&self, other: &ServiceConfig) -> bool {
        !self.eq(other)
    }
}

impl OptionalServiceConfig {
    /// TODO: (2) Implement the update_config function to update config
    /// Field list matches the fields in the ServiceConfig struct, but all of them are optional
    /// if a field is Some, it means we want to update it.
    /// You can return here anything the service needs
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.as_ref())?;
        }

        if let Some(splits) = self.splits {
            // TODO: Verify splits are valid
            config.splits = splits;
        }
        Ok(())
    }
}

#[cw_serde]

pub struct Config {
    pub input_addr: Addr,
    pub splits: SplitsConfig,
}

pub type SplitsConfig = BTreeMap<String, BTreeSet<(ServiceAccountType, Uint128)>>;
