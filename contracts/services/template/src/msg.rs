use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType,
};

use crate::CONFIG2;

#[cw_serde]
pub enum FunctionMsgs {
    NoOp {},
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

/// Everything a service needs as a parameter to be instantiated goes into `ServiceConfig`
/// `ValenceServiceInterface` generates `ServiceConfigUpdate` is used in update method that allows to update the service configuration
/// `ServiceConfigUpdate` turns all fields <T> from `ServiceConfig` into Option<T>
///  
/// Fields that are Option<T>, will be generated as OptionUpdate<T>
/// If a field cannot or should not be updated, it should be annotated with #[skip_update]
#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    /// We ignore this field when generating the ValenceServiceInterface
    /// This means this field is not updatable
    #[skip_update]
    pub skip_update_admin: ServiceAccountType,
    pub optional: Option<String>,
    pub optional2: String,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, _api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        Ok(Config {
            admin: self.skip_update_admin.to_addr(deps.api)?,
            optional: self.optional.clone(),
        })
    }
}

impl ServiceConfigUpdate {
    /// Service developer must not forget to update config storage needed
    pub fn update_config(self, deps: DepsMut) -> Result<(), ServiceError> {
        let mut config: Config = valence_service_base::load_config(deps.storage)?;

        if let OptionUpdate::Set(optional) = self.optional {
            config.optional = optional;
        }

        // While we get &mut Config, we can execute regular storage operations
        let mut config2 = CONFIG2.load(deps.storage)?;
        if let Some(optional2) = self.optional2 {
            config2.optional2 = optional2;
        }

        valence_service_base::save_config(deps.storage, &config)?;
        CONFIG2.save(deps.storage, &config2)?;

        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub optional: Option<String>,
}

/// While we can save everything in ServiceConfig into Config
/// The service is free to define its own storage struct
#[cw_serde]
pub struct Config2 {
    pub optional2: String,
}
