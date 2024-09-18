use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_denom::{CheckedDenom, DenomError, UncheckedDenom};
use cw_utils::Duration;
use getset::{Getters, Setters};
use service_base::{msg::ServiceConfigValidation, ServiceError};
use service_utils::{ServiceAccountType, ServiceConfigInterface};
use std::collections::HashMap;
use valence_macros::OptionalStruct;

#[cw_serde]
pub enum ActionsMsgs {
    Forward { execution_id: Option<u64> },
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

impl From<Duration> for ForwardingConstraints {
    fn from(min_interval: Duration) -> Self {
        ForwardingConstraints {
            min_interval: Some(min_interval),
        }
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
    pub input_addr: ServiceAccountType,
    pub output_addr: String,
    pub forwarding_configs: Vec<UncheckedForwardingConfig>,
    pub forwarding_constraints: ForwardingConstraints,
}

impl ServiceConfig {
    pub fn new(
        input_addr: ServiceAccountType,
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

        // Ensure denoms are unique in forwarding configs
        let mut denom_map: HashMap<String, ()> = HashMap::new();
        for cfc in &checked_fwd_configs {
            let key = format!("{:?}", cfc.denom);
            if denom_map.contains_key(&key) {
                return Err(ServiceError::ConfigurationError(format!(
                    "Duplicate denom '{}' in forwarding config.",
                    cfc.denom
                )));
            }
            denom_map.insert(key, ());
        }

        Ok(Config {
            input_addr: self.input_addr.to_addr(deps)?,
            output_addr: deps.api.addr_validate(&self.output_addr)?,
            forwarding_configs: checked_fwd_configs,
            forwarding_constraints: self.forwarding_constraints.clone(),
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
    pub fn update_config(self, _deps: &DepsMut, _config: &mut Config) -> Result<(), ServiceError> {
        todo!();
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
