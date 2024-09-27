use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use getset::{Getters, Setters};
use valence_macros::OptionalStruct;
use valence_service_utils::denoms::CheckedDenom;
use valence_service_utils::ServiceConfigInterface;
use valence_service_utils::{
    denoms::UncheckedDenom, error::ServiceError, msg::ServiceConfigValidation,
};

#[cw_serde]
pub enum ActionMsgs {
    Split {},
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {
    /// Query to get the processor address.
    #[returns(Addr)]
    GetProcessor {},
    /// Query to get the service configuration.
    #[returns(Config)]
    GetServiceConfig {},
}

pub type SplitConfigs = Vec<SplitConfig>;

#[cw_serde]
#[derive(Getters, Setters)]
pub struct SplitConfig {
    #[getset(get = "pub", set)]
    denom: CheckedDenom,
    #[getset(get = "pub", set)]
    account: Addr,
    #[getset(get = "pub", set)]
    amount: Option<Uint128>,
    #[getset(get = "pub", set)]
    ratio: Option<RatioConfig>,
    #[getset(get = "pub", set)]
    factor: Option<u64>,
}

impl SplitConfig {
    pub fn new(
        denom: CheckedDenom,
        account: Addr,
        amount: Option<Uint128>,
        ratio: Option<RatioConfig>,
        factor: Option<u64>,
    ) -> Self {
        SplitConfig {
            denom,
            account,
            amount,
            ratio,
            factor,
        }
    }
}

#[cw_serde]
pub enum RatioConfig {
    FixedRatio(Decimal),
    DynamicRatio { contract_addr: Addr, params: String },
}

#[cw_serde]
pub enum UncheckedRatioConfig {
    FixedRatio(Decimal),
    DynamicRatio {
        contract_addr: String,
        params: String,
    },
}

#[cw_serde]
pub struct UncheckedSplitConfig {
    denom: UncheckedDenom,
    account: String,
    amount: Option<Uint128>,
    ratio: Option<UncheckedRatioConfig>,
    factor: Option<u64>,
}

impl UncheckedSplitConfig {
    pub fn new(
        denom: UncheckedDenom,
        account: String,
        amount: Option<Uint128>,
        ratio: Option<UncheckedRatioConfig>,
        factor: Option<u64>,
    ) -> Self {
        UncheckedSplitConfig {
            denom,
            account,
            amount,
            ratio,
            factor,
        }
    }

    pub fn with_native_amount(amount: u128, denom: &str, output: &Addr) -> Self {
        UncheckedSplitConfig {
            denom: UncheckedDenom::Native(denom.to_string()),
            account: output.to_string(),
            amount: Some(amount.into()),
            ratio: None,
            factor: None,
        }
    }

    pub fn with_cw20_amount(amount: u128, addr: &Addr, output: &Addr) -> Self {
        UncheckedSplitConfig {
            denom: UncheckedDenom::Cw20(addr.to_string()),
            account: output.to_string(),
            amount: Some(amount.into()),
            ratio: None,
            factor: None,
        }
    }

    pub fn with_fixed_ratio(ratio: Decimal, denom: &str, output: &Addr) -> Self {
        UncheckedSplitConfig {
            denom: UncheckedDenom::Native(denom.to_string()),
            account: output.to_string(),
            amount: None,
            ratio: Some(UncheckedRatioConfig::FixedRatio(ratio)),
            factor: None,
        }
    }
}

#[allow(dead_code)]
struct DynamicRatioResponse {
    ratio: Uint128,
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    input_addr: String,
    splits: Vec<UncheckedSplitConfig>,
    base_denom: UncheckedDenom,
}

impl ServiceConfig {
    pub fn new(
        input_addr: String,
        splits: Vec<UncheckedSplitConfig>,
        base_denom: UncheckedDenom,
    ) -> Self {
        ServiceConfig {
            input_addr,
            splits,
            base_denom,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, ServiceError> {
        let input_addr = api.addr_validate(&self.input_addr)?;
        // Ensure denoms are unique in split configs
        ensure_split_uniqueness(&self.splits)?;
        validate_splits(api, &self.splits, &self.base_denom)?;
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
        // Convert the unchecked denoms to checked denoms
        let checked_splits = convert_to_checked_configs(deps, &self.splits)?;
        let base_denom = self
            .base_denom
            .clone()
            .into_checked(deps)
            .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?;

        Ok(Config {
            input_addr,
            splits: checked_splits,
            base_denom,
        })
    }
}

/// Ensure splits are unique in split configs
fn ensure_split_uniqueness(splits: &Vec<UncheckedSplitConfig>) -> Result<(), ServiceError> {
    let mut denom_map: HashMap<String, ()> = HashMap::new();
    for cfg in splits {
        let key = format!("{:?}|{}", cfg.denom, cfg.account);
        if denom_map.contains_key(&key) {
            return Err(ServiceError::ConfigurationError(format!(
                "Duplicate split '{:?}|{}' in split config.",
                cfg.denom, cfg.account
            )));
        }
        denom_map.insert(key, ());
    }
    Ok(())
}

fn convert_to_checked_configs(
    deps: Deps<'_>,
    splits: &[UncheckedSplitConfig],
) -> Result<Vec<SplitConfig>, ServiceError> {
    splits
        .iter()
        .map(|c| {
            let denom = c
                .denom
                .clone()
                .into_checked(deps)
                .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?;
            let account = deps.api.addr_validate(&c.account)?;
            let ratio = c
                .ratio
                .as_ref()
                .map(|r| convert_to_checked_ratio_config(deps.api, r))
                .transpose()?;

            Ok(SplitConfig {
                denom,
                account,
                amount: c.amount,
                ratio,
                factor: c.factor,
            })
        })
        .collect()
}

fn convert_to_checked_ratio_config(
    api: &dyn cosmwasm_std::Api,
    ratio: &UncheckedRatioConfig,
) -> Result<RatioConfig, ServiceError> {
    match ratio {
        UncheckedRatioConfig::FixedRatio(r) => Ok(RatioConfig::FixedRatio(*r)),
        UncheckedRatioConfig::DynamicRatio {
            contract_addr,
            params,
        } => Ok(RatioConfig::DynamicRatio {
            contract_addr: api.addr_validate(contract_addr)?,
            params: params.clone(),
        }),
    }
}

impl ServiceConfigInterface<ServiceConfig> for ServiceConfig {
    /// This function is used to see if 2 configs are different
    fn is_diff(&self, other: &ServiceConfig) -> bool {
        !self.eq(other)
    }
}

impl OptionalServiceConfig {
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
        if let Some(input_addr) = self.input_addr {
            config.input_addr = deps.api.addr_validate(&input_addr)?;
        }

        if let Some(_splits) = self.splits {
            // validate_splits(&splits)?;
            todo!();
            // config.splits = splits;
        }
        Ok(())
    }
}

fn validate_splits(
    api: &dyn cosmwasm_std::Api,
    splits: &Vec<UncheckedSplitConfig>,
    base_denom: &UncheckedDenom,
) -> Result<(), ServiceError> {
    if splits.is_empty() {
        return Err(ServiceError::ConfigurationError(
            "No split configuration provided.".to_string(),
        ));
    }

    for split in splits {
        api.addr_validate(&split.account)?;
        // Note: can't validate denom without the deps

        if !(split.amount.is_some() || split.ratio.is_some()) {
            return Err(ServiceError::ConfigurationError(
                "Invalid split config: should specify either an amount or a ratio.".to_string(),
            ));
        }

        if split.amount.is_some() && split.factor.is_some() {
            return Err(ServiceError::ConfigurationError(
                "Invalid split config: a factor cannot be specified with an amount.".to_string(),
            ));
        }

        // Verify base denom split config
        if split.denom == *base_denom {
            match (split.amount, split.ratio.clone()) {
                (Some(_), None) => {
                    // Valid config: specific amount configured for base denom
                }
                (None, Some(UncheckedRatioConfig::FixedRatio(ratio)))
                    if ratio == Decimal::one() =>
                {
                    // Valid config: fixed ratio of 1 configured for base denom
                }
                _ => {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid split config: fixed ratio for base denom must be 1.".to_string(),
                    ));
                }
            }
        }

        if let Some(UncheckedRatioConfig::DynamicRatio {
            contract_addr,
            params: _,
        }) = &split.ratio
        {
            api.addr_validate(contract_addr)?;
        }
    }

    // If there are ratios, we only allow an amount to be set for the base denom
    if splits.iter().any(|s| s.ratio.is_some())
        && splits
            .iter()
            .any(|s| s.amount.is_some() && s.denom != *base_denom)
    {
        return Err(ServiceError::ConfigurationError(
            "Invalid split config: only base denom can have an amount when ratios are specified for other some denoms.".to_string(),
        ));
    }

    Ok(())
}

#[cw_serde]
#[derive(Getters, Setters)]
pub struct Config {
    #[getset(get = "pub", set)]
    input_addr: Addr,
    #[getset(get = "pub", set)]
    splits: SplitConfigs,
    #[getset(get = "pub", set)]
    base_denom: CheckedDenom,
}

impl Config {
    pub fn new(input_addr: Addr, splits: SplitConfigs, base_denom: CheckedDenom) -> Self {
        Config {
            input_addr,
            splits,
            base_denom,
        }
    }
}
