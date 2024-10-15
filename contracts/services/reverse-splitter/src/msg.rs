use std::collections::HashSet;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use getset::{Getters, Setters};
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::denoms::CheckedDenom;
use valence_service_utils::ServiceAccountType;
use valence_service_utils::{
    denoms::UncheckedDenom, error::ServiceError, msg::ServiceConfigValidation,
};

#[cw_serde]
pub enum ActionMsgs {
    Split {},
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

pub type SplitConfigs = Vec<SplitConfig>;

#[cw_serde]
#[derive(Getters, Setters)]
pub struct SplitConfig {
    #[getset(get = "pub", set)]
    denom: CheckedDenom,
    #[getset(get = "pub", set)]
    account: Addr,
    #[getset(get = "pub", set)]
    amount: SplitAmount,
    #[getset(get = "pub", set)]
    factor: Option<u64>,
}

impl SplitConfig {
    pub fn new(
        denom: CheckedDenom,
        account: Addr,
        amount: SplitAmount,
        factor: Option<u64>,
    ) -> Self {
        SplitConfig {
            denom,
            account,
            amount,
            factor,
        }
    }
}

#[cw_serde]
pub enum SplitAmount {
    FixedAmount(Uint128),
    FixedRatio(Decimal),
    DynamicRatio { contract_addr: Addr, params: String },
}

#[cw_serde]
pub enum UncheckedSplitAmount {
    FixedAmount(Uint128),
    FixedRatio(Decimal),
    DynamicRatio {
        contract_addr: String,
        params: String,
    },
}

#[cw_serde]
pub struct UncheckedSplitConfig {
    pub denom: UncheckedDenom,
    pub account: ServiceAccountType,
    pub amount: UncheckedSplitAmount,
    pub factor: Option<u64>,
}

impl UncheckedSplitConfig {
    pub fn new(
        denom: UncheckedDenom,
        account: impl Into<ServiceAccountType>,
        amount: UncheckedSplitAmount,
        factor: Option<u64>,
    ) -> Self {
        UncheckedSplitConfig {
            denom,
            account: account.into(),
            amount,
            factor,
        }
    }

    pub fn with_native_amount(amount: u128, denom: &str, input: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Native(denom.to_string()),
            input,
            UncheckedSplitAmount::FixedAmount(amount.into()),
            None,
        )
    }

    pub fn with_cw20_amount(amount: u128, addr: &Addr, input: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Cw20(addr.to_string()),
            input,
            UncheckedSplitAmount::FixedAmount(amount.into()),
            None,
        )
    }

    pub fn with_native_ratio(ratio: Decimal, denom: &str, input: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Native(denom.to_string()),
            input,
            UncheckedSplitAmount::FixedRatio(ratio),
            None,
        )
    }

    pub fn with_cw20_ratio(ratio: Decimal, addr: &Addr, input: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Cw20(addr.to_string()),
            input,
            UncheckedSplitAmount::FixedRatio(ratio),
            None,
        )
    }

    pub fn with_native_dyn_ratio(
        contract_addr: &Addr,
        params: &str,
        denom: &str,
        input: &Addr,
    ) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Native(denom.to_string()),
            input,
            UncheckedSplitAmount::DynamicRatio {
                contract_addr: contract_addr.to_string(),
                params: params.to_string(),
            },
            None,
        )
    }

    pub fn with_cw20_dyn_ratio(
        contract_addr: &Addr,
        params: &str,
        addr: &Addr,
        input: &Addr,
    ) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Cw20(addr.to_string()),
            input,
            UncheckedSplitAmount::DynamicRatio {
                contract_addr: contract_addr.to_string(),
                params: params.to_string(),
            },
            None,
        )
    }

    pub fn set_factor(mut self, factor: u64) -> Self {
        self.factor = Some(factor);
        self
    }
}

#[allow(dead_code)]
struct DynamicRatioResponse {
    ratio: Uint128,
}

#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    pub output_addr: ServiceAccountType,
    pub splits: Vec<UncheckedSplitConfig>,
    pub base_denom: UncheckedDenom,
}

impl ServiceConfig {
    pub fn new(
        output_addr: impl Into<ServiceAccountType>,
        splits: Vec<UncheckedSplitConfig>,
        base_denom: UncheckedDenom,
    ) -> Self {
        ServiceConfig {
            output_addr: output_addr.into(),
            splits,
            base_denom,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, ServiceError> {
        let output_addr = self.output_addr.to_addr(api)?;
        validate_splits(api, &self.splits, &self.base_denom)?;
        Ok(output_addr)
    }
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let output_addr = self.do_validate(deps.api)?;
        // Convert the unchecked denoms to checked denoms
        let checked_splits = convert_to_checked_configs(deps, &self.splits)?;
        let base_denom = self
            .base_denom
            .clone()
            .into_checked(deps)
            .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?;

        Ok(Config {
            output_addr,
            splits: checked_splits,
            base_denom,
        })
    }
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
            let account = c.account.to_addr(deps.api)?;
            let amount = convert_to_checked_split_amount(deps.api, &c.amount)?;

            Ok(SplitConfig {
                denom,
                account,
                amount,
                factor: c.factor,
            })
        })
        .collect()
}

fn convert_to_checked_split_amount(
    api: &dyn cosmwasm_std::Api,
    amount: &UncheckedSplitAmount,
) -> Result<SplitAmount, ServiceError> {
    match amount {
        UncheckedSplitAmount::FixedAmount(a) => Ok(SplitAmount::FixedAmount(*a)),
        UncheckedSplitAmount::FixedRatio(r) => Ok(SplitAmount::FixedRatio(*r)),
        UncheckedSplitAmount::DynamicRatio {
            contract_addr,
            params,
        } => Ok(SplitAmount::DynamicRatio {
            contract_addr: api.addr_validate(contract_addr)?,
            params: params.clone(),
        }),
    }
}

impl ServiceConfigUpdate {
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
        // First update output_addr & base_denom (if needed)
        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_addr(deps.api)?;
        }

        if let Some(base_denom) = self.base_denom.clone() {
            config.base_denom = base_denom
                .into_checked(deps.as_ref())
                .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?;
        }

        // Then validate & update splits (if needed)
        if let Some(splits) = self.splits {
            validate_splits(
                deps.api,
                &splits,
                // Use the new base_denom if it was updated, or the existing one (as unchecked)
                &self.base_denom.unwrap_or(match &config.base_denom {
                    CheckedDenom::Native(denom) => UncheckedDenom::Native(denom.clone()),
                    CheckedDenom::Cw20(addr) => UncheckedDenom::Cw20(addr.to_string()),
                }),
            )?;

            config.splits = convert_to_checked_configs(deps.as_ref(), &splits)?;
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

    let mut denom_set = HashSet::new();
    let mut config_has_ratio = false;
    let mut config_has_amount_for_non_base_denom = false;
    for split in splits {
        split.account.to_addr(api)?;
        // Note: can't validate denom without the deps

        // Ensure splits are unique in split configs
        let key = format!("{:?}|{:?}", split.denom, split.account);
        if !denom_set.insert(key) {
            return Err(ServiceError::ConfigurationError(format!(
                "Duplicate split '{:?}|{:?}' in split config.",
                split.denom, split.account
            )));
        }

        match &split.amount {
            UncheckedSplitAmount::FixedAmount(amount) => {
                if amount == Uint128::zero() {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid split config: amount cannot be zero.".to_string(),
                    ));
                }

                if split.factor.is_some() {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid split config: a factor cannot be specified with an amount."
                            .to_string(),
                    ));
                }

                if split.denom != *base_denom {
                    config_has_amount_for_non_base_denom = true;
                }
            }
            UncheckedSplitAmount::FixedRatio(ratio) => {
                if ratio == Decimal::zero() {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid split config: ratio cannot be zero.".to_string(),
                    ));
                }
                if split.denom == *base_denom && ratio != Decimal::one() {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid split config: fixed ratio for base denom must be 1.".to_string(),
                    ));
                }
                config_has_ratio = true;
            }
            UncheckedSplitAmount::DynamicRatio { contract_addr, .. } => {
                api.addr_validate(contract_addr)?;
                if split.denom == *base_denom {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid split config: ratio for base denom cannot be a dynamic one."
                            .to_string(),
                    ));
                }
                config_has_ratio = true;
            }
        }

        if let Some(factor) = split.factor {
            if factor == 0 {
                return Err(ServiceError::ConfigurationError(
                    "Invalid split config: factor cannot be zero.".to_string(),
                ));
            }
        }
    }

    // If there are ratios, we only allow an amount to be set for the base denom
    if config_has_ratio && config_has_amount_for_non_base_denom {
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
    output_addr: Addr,
    #[getset(get = "pub", set)]
    splits: SplitConfigs,
    #[getset(get = "pub", set)]
    base_denom: CheckedDenom,
}

impl Config {
    pub fn new(output_addr: Addr, splits: SplitConfigs, base_denom: CheckedDenom) -> Self {
        Config {
            output_addr,
            splits,
            base_denom,
        }
    }
}
