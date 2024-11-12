use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use cw_utils::Duration;
use getset::{Getters, Setters};
use std::collections::HashMap;
use valence_library_utils::{
    denoms::{CheckedDenom, DenomError, UncheckedDenom},
    error::LibraryError,
    msg::LibraryConfigValidation,
    LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
/// Enum representing the different function messages that can be sent.
pub enum FunctionMsgs {
    /// Message to forward tokens.
    Forward {},
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

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
            max_amount: max_amount.into(),
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
            max_amount: max_amount.into(),
        }
    }
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
/// Struct representing the library configuration.
pub struct LibraryConfig {
    /// The input address for the library.
    pub input_addr: LibraryAccountType,
    /// The output address for the library.
    pub output_addr: LibraryAccountType,
    /// The forwarding configurations for the library.
    pub forwarding_configs: Vec<UncheckedForwardingConfig>,
    /// The forwarding constraints for the library.
    pub forwarding_constraints: ForwardingConstraints,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        forwarding_configs: Vec<UncheckedForwardingConfig>,
        forwarding_constraints: ForwardingConstraints,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            forwarding_configs,
            forwarding_constraints,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        // Ensure denoms are unique in forwarding configs
        ensure_denom_uniqueness(&self.forwarding_configs)?;
        Ok((input_addr, output_addr))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr) = self.do_validate(deps.api)?;

        // Convert the unchecked denoms to checked denoms
        let checked_fwd_configs = convert_to_checked_configs(&self.forwarding_configs, deps)?;

        Ok(Config::new(
            input_addr,
            output_addr,
            checked_fwd_configs,
            self.forwarding_constraints.clone(),
        ))
    }
}

/// Ensure denoms are unique in forwarding configs
fn ensure_denom_uniqueness(
    fwd_configs: &Vec<UncheckedForwardingConfig>,
) -> Result<(), LibraryError> {
    let mut denom_map: HashMap<String, ()> = HashMap::new();
    for cfg in fwd_configs {
        let key = format!("{:?}", cfg.denom);
        if denom_map.contains_key(&key) {
            return Err(LibraryError::ConfigurationError(format!(
                "Duplicate denom '{:?}' in forwarding config.",
                cfg.denom
            )));
        }
        denom_map.insert(key, ());
    }
    Ok(())
}

fn convert_to_checked_configs(
    fwd_configs: &[UncheckedForwardingConfig],
    deps: Deps<'_>,
) -> Result<Vec<ForwardingConfig>, LibraryError> {
    fwd_configs
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
        .map_err(|err| LibraryError::ConfigurationError(err.to_string()))
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_addr(deps.api)?;
        }

        if let Some(forwarding_configs) = self.forwarding_configs {
            // Ensure denoms are unique in forwarding configs
            ensure_denom_uniqueness(&forwarding_configs)?;

            // Convert the unchecked denoms to checked denoms
            let checked_fwd_configs =
                convert_to_checked_configs(&forwarding_configs, deps.as_ref())?;

            config.forwarding_configs = checked_fwd_configs;
        }

        if let Some(forwarding_constraints) = self.forwarding_constraints {
            config.forwarding_constraints = forwarding_constraints;
        }

        valence_library_base::save_config(deps.storage, &config)?;

        Ok(())
    }
}

#[cw_serde]
#[derive(Getters, Setters)]
/// Struct representing the validated library configuration.
pub struct Config {
    /// The input address for the library.
    #[getset(get = "pub", set)]
    input_addr: Addr,
    /// The output address for the library.
    #[getset(get = "pub", set)]
    output_addr: Addr,
    /// The forwarding configurations for the library.
    #[getset(get = "pub", set)]
    forwarding_configs: ForwardingConfigs,
    /// The forwarding constraints for the library.
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
