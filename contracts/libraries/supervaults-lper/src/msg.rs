use std::fmt::Display;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use neutron_std::types::neutron::util::precdec::PrecDec;
use valence_library_utils::{
    error::LibraryError, liquidity_utils::AssetData, msg::LibraryConfigValidation,
    LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub vault_addr: Addr,
    pub lp_config: LiquidityProviderConfig,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    pub vault_addr: String,
    pub lp_config: LiquidityProviderConfig,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        vault_addr: String,
        lp_config: LiquidityProviderConfig,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            vault_addr,
            lp_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let vault_addr = api.addr_validate(&self.vault_addr)?;

        Ok((input_addr, output_addr, vault_addr))
    }
}

#[cw_serde]
pub struct CombinedPriceResponse {
    pub token_0_price: PrecDec,
    pub token_1_price: PrecDec,
    pub price_0_to_1: PrecDec,
}

#[cw_serde]
pub struct PrecDecimalRange {
    pub min: PrecDec,
    pub max: PrecDec,
}

impl Display for PrecDecimalRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.min, self.max)
    }
}

#[cw_serde]
pub enum FunctionMsgs {
    ProvideLiquidity {
        expected_vault_ratio_range: Option<PrecDecimalRange>,
    },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct LiquidityProviderConfig {
    pub asset_data: AssetData,
    pub lp_denom: String,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, vault_addr) = self.do_validate(deps.api)?;

        ensure_correct_vault(deps, vault_addr.to_string(), &self.lp_config)?;

        Ok(Config {
            input_addr,
            output_addr,
            vault_addr,
            lp_config: self.lp_config.clone(),
        })
    }
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

        if let Some(vault_addr) = self.vault_addr {
            config.vault_addr = deps.api.addr_validate(&vault_addr)?;
        }

        if let Some(lp_config) = self.lp_config {
            config.lp_config = lp_config;
        }

        ensure_correct_vault(
            deps.as_ref(),
            config.vault_addr.to_string(),
            &config.lp_config,
        )?;

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

fn ensure_correct_vault(
    deps: Deps,
    vault_addr: String,
    lp_config: &LiquidityProviderConfig,
) -> Result<(), LibraryError> {
    let vault_config: valence_supervaults_utils::state::Config = deps.querier.query_wasm_smart(
        vault_addr,
        &valence_supervaults_utils::msg::QueryMsg::GetConfig {},
    )?;

    ensure!(
        lp_config.asset_data.asset1 == vault_config.pair_data.token_0.denom
            && lp_config.asset_data.asset2 == vault_config.pair_data.token_1.denom,
        LibraryError::ConfigurationError(
            "Pool type does not match the expected pair type".to_string(),
        )
    );

    ensure!(
        vault_config.lp_denom == lp_config.lp_denom,
        LibraryError::ConfigurationError(format!(
            "Vault LP denom mismatch; expected: {}, got {}",
            lp_config.lp_denom, vault_config.lp_denom
        ))
    );

    Ok(())
}
