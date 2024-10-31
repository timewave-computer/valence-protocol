use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::ServiceAccountType;
use valence_service_utils::{error::ServiceError, msg::ServiceConfigValidation};

#[cw_serde]
pub enum ActionMsgs {
    Tokenize {},
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    // map of denom to input ratio, e.g.
    // { "atom": 1, "usdc": 10 } would mean that
    // each tokenized output position would require
    // 1 atom and 10 usdc
    pub input_denoms: BTreeMap<String, Decimal>,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        output_addr: Addr,
        input_denoms: BTreeMap<String, Decimal>,
    ) -> Self {
        Config {
            input_addr,
            output_addr,
            input_denoms,
        }
    }
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub output_addr: ServiceAccountType,
    pub input_denoms: BTreeMap<String, Decimal>,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        output_addr: impl Into<ServiceAccountType>,
        input_denoms: BTreeMap<String, Decimal>,
    ) -> Self {
        ServiceConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            input_denoms,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, BTreeMap<String, Decimal>), ServiceError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.input_addr.to_addr(api)?;

        Ok((input_addr, output_addr, self.input_denoms.clone()))
    }
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let (input_addr, output_addr, map) = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            input_denoms: map,
        })
    }
}

impl ServiceConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), ServiceError> {
        let mut config: Config = valence_service_base::load_config(deps.storage)?;

        // First update input_addr (if needed)
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        valence_service_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}
