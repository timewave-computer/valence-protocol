use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_utils::Duration;
use getset::{Getters, Setters};
use std::collections::HashMap;
use valence_macros::OptionalStruct;
use valence_service_base::{msg::ServiceConfigValidation, ServiceError};
use valence_service_utils::{
    denoms::{CheckedDenom, DenomError, UncheckedDenom},
    ServiceConfigInterface,
};

#[cw_serde]
pub enum ActionsMsgs {
    Forward {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetOwner {},
    #[returns(Addr)]
    GetProcessor {},
    #[returns(Config)]
    GetServiceConfig {},
}

// Forwarding configuration per denom
type ForwardingConfigs = Vec<ForwardingConfig>;

// Defines the max amount of tokens to be forwarded per time period for a given denom
#[cw_serde]
#[derive(Getters, Setters)]
pub struct ForwardingConfig {
    #[getset(get = "pub", set)]
    denom: CheckedDenom,
    #[getset(get = "pub", set)]
    max_amount: Uint128, // Max amount of tokens to be transferred per Forward operation
}

impl From<(CheckedDenom, u128)> for ForwardingConfig {
    fn from((denom, max_amount): (CheckedDenom, u128)) -> Self {
        ForwardingConfig {
            denom,
            max_amount: Uint128::from(max_amount),
        }
    }
}

// Time constraints on forwarding operations
#[cw_serde]
#[derive(Getters, Setters, Default)]
pub struct ForwardingConstraints {
    #[getset(get = "pub", set)]
    min_interval: Option<Duration>,
}

impl ForwardingConstraints {
    pub fn new(min_interval: Option<Duration>) -> Self {
        ForwardingConstraints { min_interval }
    }
}

#[cw_serde]
pub struct UncheckedForwardingConfig {
    pub denom: UncheckedDenom,
    pub max_amount: Uint128,
}

impl From<(UncheckedDenom, u128)> for UncheckedForwardingConfig {
    fn from((denom, max_amount): (UncheckedDenom, u128)) -> Self {
        UncheckedForwardingConfig {
            denom,
            max_amount: Uint128::from(max_amount),
        }
    }
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    pub input_addr: String,
    pub output_addr: String,
    pub forwarding_configs: Vec<UncheckedForwardingConfig>,
    pub forwarding_constraints: ForwardingConstraints,
}

impl ServiceConfig {
    pub fn new(
        input_addr: String,
        output_addr: String,
        forwarding_configs: Vec<UncheckedForwardingConfig>,
        forwarding_constraints: ForwardingConstraints,
    ) -> Self {
        ServiceConfig {
            input_addr,
            output_addr,
            forwarding_configs,
            forwarding_constraints,
        }
    }
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let input_addr = deps.api.addr_validate(&self.input_addr)?;
        let output_addr = deps.api.addr_validate(&self.output_addr)?;
        // Ensure denoms are unique in forwarding configs
        ensure_denom_uniqueness(&self.forwarding_configs)?;

        // Convert the unchecked denoms to checked denoms
        let checked_fwd_configs = self
            .forwarding_configs
            .iter()
            .map(|ufc| {
                ufc.denom
                    .clone()
                    .into_checked(deps)
                    .map(|checked| ForwardingConfig {
                        denom: checked,
                        max_amount: ufc.max_amount,
                    })
            })
            .collect::<Result<Vec<ForwardingConfig>, DenomError>>()
            .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?;

        Ok(Config {
            input_addr,
            output_addr,
            forwarding_configs: checked_fwd_configs,
            forwarding_constraints: self.forwarding_constraints.clone(),
        })
    }
}

fn ensure_denom_uniqueness(
    checked_fwd_configs: &Vec<UncheckedForwardingConfig>,
) -> Result<(), ServiceError> {
    let mut denom_map: HashMap<String, ()> = HashMap::new();
    for ufc in checked_fwd_configs {
        let key = format!("{:?}", ufc.denom);
        if denom_map.contains_key(&key) {
            return Err(ServiceError::ConfigurationError(format!(
                "Duplicate denom '{:?}' in forwarding config.",
                ufc.denom
            )));
        }
        denom_map.insert(key, ());
    }
    Ok(())
}

impl ServiceConfigInterface<ServiceConfig> for ServiceConfig {
    /// This function is used to see if 2 configs are different
    fn is_diff(&self, other: &ServiceConfig) -> bool {
        !self.eq(other)
    }
}

impl OptionalServiceConfig {
    pub fn update_config(self, _deps: &DepsMut, _config: &mut Config) -> Result<(), ServiceError> {
        //TODO: Implement update_config
        Ok(())
    }
}

#[cw_serde]
#[derive(Getters, Setters)]
pub struct Config {
    #[getset(get = "pub", set)]
    input_addr: Addr,
    #[getset(get = "pub", set)]
    output_addr: Addr,
    #[getset(get = "pub", set)]
    forwarding_configs: ForwardingConfigs,
    #[getset(get = "pub", set)]
    forwarding_constraints: ForwardingConstraints,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        output_addr: Addr,
        forwarding_configs: Vec<ForwardingConfig>,
        forwarding_constraints: ForwardingConstraints,
    ) -> Self {
        Config {
            input_addr,
            output_addr,
            forwarding_configs,
            forwarding_constraints,
        }
    }
}
