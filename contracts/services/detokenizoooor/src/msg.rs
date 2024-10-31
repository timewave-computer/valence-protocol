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
    pub input_addr: String,
    pub voucher_denom: String,
    pub redeemable_denoms: HashSet<String>,
}

impl DetokenizoooorConfig {
    pub fn new(
        input_addr: String,
        voucher_denom: String,
        redeemable_denoms: HashSet<String>,
    ) -> Self {
        DetokenizoooorConfig {
            input_addr,
            voucher_denom,
            redeemable_denoms,
        }
    }
}

#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub voucher_denom: String,
    pub detokenizoooor_config: DetokenizoooorConfig,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        voucher_denom: String,
        detokenizoooor_config: DetokenizoooorConfig,
    ) -> Self {
        ServiceConfig {
            input_addr: input_addr.into(),
            voucher_denom,
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

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let input_addr = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            voucher_denom: self.voucher_denom.clone(),
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
    pub input_addr: Addr,
    pub voucher_denom: String,
    pub detokenizoooor_config: DetokenizoooorConfig,
}
