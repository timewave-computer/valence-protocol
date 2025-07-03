use std::collections::{HashMap, HashSet};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use getset::{Getters, Setters};
use valence_library_utils::denoms::CheckedDenom;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{
    denoms::UncheckedDenom, error::LibraryError, msg::LibraryConfigValidation,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    Split {},
}

#[valence_library_query]
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
}

#[cw_serde]
pub enum SplitAmount {
    FixedAmount(Uint128),
    FixedRatio(Decimal),
    DynamicRatio { contract_addr: Addr, params: String },
}

impl SplitConfig {
    pub fn new(denom: CheckedDenom, account: Addr, amount: SplitAmount) -> Self {
        SplitConfig {
            denom,
            account,
            amount,
        }
    }
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
    pub account: LibraryAccountType,
    pub amount: UncheckedSplitAmount,
}

impl UncheckedSplitConfig {
    pub fn new(
        denom: UncheckedDenom,
        account: impl Into<LibraryAccountType>,
        amount: UncheckedSplitAmount,
    ) -> Self {
        UncheckedSplitConfig {
            denom,
            account: account.into(),
            amount,
        }
    }

    pub fn with_native_amount(amount: u128, denom: &str, output: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Native(denom.to_string()),
            output,
            UncheckedSplitAmount::FixedAmount(amount.into()),
        )
    }

    pub fn with_cw20_amount(amount: u128, addr: &Addr, output: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Cw20(addr.to_string()),
            output,
            UncheckedSplitAmount::FixedAmount(amount.into()),
        )
    }

    pub fn with_native_ratio(ratio: Decimal, denom: &str, output: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Native(denom.to_string()),
            output,
            UncheckedSplitAmount::FixedRatio(ratio),
        )
    }

    pub fn with_cw20_ratio(ratio: Decimal, addr: &Addr, output: &Addr) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Cw20(addr.to_string()),
            output,
            UncheckedSplitAmount::FixedRatio(ratio),
        )
    }

    pub fn with_native_dyn_ratio(
        contract_addr: &Addr,
        params: &str,
        denom: &str,
        output: &Addr,
    ) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Native(denom.to_string()),
            output,
            UncheckedSplitAmount::DynamicRatio {
                contract_addr: contract_addr.to_string(),
                params: params.to_string(),
            },
        )
    }

    pub fn with_cw20_dyn_ratio(
        contract_addr: &Addr,
        params: &str,
        addr: &Addr,
        output: &Addr,
    ) -> Self {
        UncheckedSplitConfig::new(
            UncheckedDenom::Cw20(addr.to_string()),
            output,
            UncheckedSplitAmount::DynamicRatio {
                contract_addr: contract_addr.to_string(),
                params: params.to_string(),
            },
        )
    }
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub splits: Vec<UncheckedSplitConfig>,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        splits: Vec<UncheckedSplitConfig>,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            splits,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        validate_splits(api, &self.splits)?;
        Ok(input_addr)
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let input_addr = self.do_validate(deps.api)?;
        // Convert the unchecked denoms to checked denoms
        let checked_splits = convert_to_checked_configs(deps, &self.splits)?;

        Ok(Config {
            input_addr,
            splits: checked_splits,
        })
    }
}

fn convert_to_checked_configs(
    deps: Deps<'_>,
    splits: &[UncheckedSplitConfig],
) -> Result<Vec<SplitConfig>, LibraryError> {
    splits
        .iter()
        .map(|c| {
            let denom = c
                .denom
                .clone()
                .into_checked(deps)
                .map_err(|err| LibraryError::ConfigurationError(err.to_string()))?;
            let account = c.account.to_addr(deps.api)?;
            let amount = convert_to_checked_split_amount(deps.api, &c.amount)?;

            Ok(SplitConfig::new(denom, account, amount))
        })
        .collect()
}

fn convert_to_checked_split_amount(
    api: &dyn cosmwasm_std::Api,
    amount: &UncheckedSplitAmount,
) -> Result<SplitAmount, LibraryError> {
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

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        // First update input_addr (if needed)
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        // Then validate & update splits (if needed)
        if let Some(splits) = self.splits {
            validate_splits(deps.api, &splits)?;

            config.splits = convert_to_checked_configs(deps.as_ref(), &splits)?;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

fn validate_splits(
    api: &dyn cosmwasm_std::Api,
    splits: &Vec<UncheckedSplitConfig>,
) -> Result<(), LibraryError> {
    if splits.is_empty() {
        return Err(LibraryError::ConfigurationError(
            "No split configuration provided.".to_string(),
        ));
    }

    let mut denom_set = HashSet::new();
    let mut denom_amount = HashSet::new();
    let mut denom_ratios: HashMap<String, Decimal> = HashMap::new();
    for split in splits {
        split.account.to_addr(api)?;
        // Note: can't validate denom without the deps

        // Ensure splits are unique in split configs
        let key = format!("{:?}|{:?}", split.denom, split.account);
        if !denom_set.insert(key) {
            return Err(LibraryError::ConfigurationError(format!(
                "Duplicate split '{:?}|{:?}' in split config.",
                split.denom, split.account
            )));
        }

        let denom_key = format!("{:?}", split.denom);
        match &split.amount {
            UncheckedSplitAmount::FixedAmount(amount) => {
                if amount.is_zero() {
                    return Err(LibraryError::ConfigurationError(
                        "Invalid split config: amount cannot be zero.".to_string(),
                    ));
                }
                // Mark that this denom has a split amount
                denom_amount.insert(denom_key);
            }
            UncheckedSplitAmount::FixedRatio(ratio) => {
                if ratio.is_zero() {
                    return Err(LibraryError::ConfigurationError(
                        "Invalid split config: ratio cannot be zero.".to_string(),
                    ));
                }
                denom_ratios
                    .entry(denom_key)
                    .and_modify(|sum| *sum += ratio)
                    .or_insert(*ratio);
            }
            UncheckedSplitAmount::DynamicRatio { contract_addr, .. } => {
                api.addr_validate(contract_addr)?;
            }
        }
    }

    // Verify sum per denom is equal to 1 (rounded up)
    for (key, sum) in denom_ratios.iter() {
        if denom_amount.contains(key) {
            return Err(LibraryError::ConfigurationError(format!(
                "Invalid split config: cannot combine amount and ratio for the same denom '{key}'."
            )));
        }

        if sum.to_uint_ceil() != Uint128::one() {
            return Err(LibraryError::ConfigurationError(format!(
                "Invalid split config: sum of ratios for denom '{key}' is not equal to 1."
            )));
        }
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
}

impl Config {
    pub fn new(input_addr: Addr, splits: SplitConfigs) -> Self {
        Config { input_addr, splits }
    }
}
