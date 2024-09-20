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
/// Enum representing the different action messages that can be sent.
pub enum ActionsMsgs {
    /// Message to forward tokens.
    Forward {},
}

#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {
    /// Query to get the owner address.
    #[returns(Addr)]
    GetOwner {},
    /// Query to get the processor address.
    #[returns(Addr)]
    GetProcessor {},
    /// Query to get the service configuration.
    #[returns(Config)]
    GetServiceConfig {},
}

// Forwarding configuration per denom
type ForwardingConfigs = Vec<ForwardingConfig>;

#[cw_serde]
#[derive(Getters, Setters)]
/// Struct representing the forwarding configuration for a specific denom.
pub struct ForwardingConfig {
    /// The denom to be forwarded.
    #[getset(get = "pub", set)]
    denom: CheckedDenom,
    /// The maximum amount of tokens to be transferred per forward operation.
    #[getset(get = "pub", set)]
    max_amount: Uint128,
}

impl From<(CheckedDenom, u128)> for ForwardingConfig {
    fn from((denom, max_amount): (CheckedDenom, u128)) -> Self {
        ForwardingConfig {
            denom,
            max_amount: Uint128::from(max_amount),
        }
    }
}

#[cw_serde]
#[derive(Getters, Setters, Default)]
/// Struct representing the time constraints on forwarding operations.
pub struct ForwardingConstraints {
    /// The minimum interval between forwarding operations.
    #[getset(get = "pub", set)]
    min_interval: Option<Duration>,
}

impl ForwardingConstraints {
    pub fn new(min_interval: Option<Duration>) -> Self {
        ForwardingConstraints { min_interval }
    }
}

#[cw_serde]
/// Struct representing an unchecked forwarding configuration.
pub struct UncheckedForwardingConfig {
    /// The denom to be forwarded.
    pub denom: UncheckedDenom,
    /// The maximum amount of tokens to be transferred per forward operation.
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
/// Struct representing the service configuration.
pub struct ServiceConfig {
    /// The input address for the service.
    pub input_addr: String,
    /// The output address for the service.
    pub output_addr: String,
    /// The forwarding configurations for the service.
    pub forwarding_configs: Vec<UncheckedForwardingConfig>,
    /// The forwarding constraints for the service.
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

/// Ensure denoms are unique in forwarding configs
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
/// Struct representing the validated service configuration.
pub struct Config {
    /// The input address for the service.
    #[getset(get = "pub", set)]
    input_addr: Addr,
    /// The output address for the service.
    #[getset(get = "pub", set)]
    output_addr: Addr,
    /// The forwarding configurations for the service.
    #[getset(get = "pub", set)]
    forwarding_configs: ForwardingConfigs,
    /// The forwarding constraints for the service.
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
