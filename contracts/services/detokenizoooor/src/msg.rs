use std::collections::{HashMap, HashSet};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::ServiceAccountType;
use valence_service_utils::{error::ServiceError, msg::ServiceConfigValidation};

#[cw_serde]
pub enum ActionMsgs {
    Detokenize { addresses: HashSet<String> },
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

#[cw_serde]
pub struct DetokenizoooorConfig {
    pub denom: String,
    pub ratios: HashMap<String, Decimal>,
}

impl DetokenizoooorConfig {
    pub fn new(denom: String, ratios: HashMap<String, Decimal>) -> Self {
        DetokenizoooorConfig { denom, ratios }
    }
}

#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub detokenizoooor_config: DetokenizoooorConfig,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        detokenizoooor_config: DetokenizoooorConfig,
    ) -> Self {
        ServiceConfig {
            input_addr: input_addr.into(),
            detokenizoooor_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, ServiceError> {
        let input_addr = self.input_addr.to_addr(api)?;
        Ok(input_addr)
    }
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, _deps: Deps) -> Result<Config, ServiceError> {
        Ok(Config {
            detokenizoooor_config: self.detokenizoooor_config.clone(),
        })
    }
}

impl ServiceConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), ServiceError> {
        let mut config: Config = valence_service_base::load_config(deps.storage)?;
        // Update config if needed
        if let Some(detokenizoooor_config) = self.detokenizoooor_config {
            config.detokenizoooor_config = detokenizoooor_config;
        }

        valence_service_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub detokenizoooor_config: DetokenizoooorConfig,
}

impl Config {
    pub fn new(detokenizoooor_config: DetokenizoooorConfig) -> Self {
        Config {
            detokenizoooor_config,
        }
    }
}
