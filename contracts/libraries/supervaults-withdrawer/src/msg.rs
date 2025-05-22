use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{
    error::LibraryError, liquidity_utils::AssetData, msg::LibraryConfigValidation,
    LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};
use valence_supervaults_utils::{ensure_correct_vault, prec_dec_range::PrecDecimalRange};

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub vault_addr: Addr,
    pub lw_config: LiquidityWithdrawerConfig,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    pub vault_addr: String,
    pub lw_config: LiquidityWithdrawerConfig,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        vault_addr: String,
        lw_config: LiquidityWithdrawerConfig,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            vault_addr,
            lw_config,
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
pub enum FunctionMsgs {
    WithdrawLiquidity {
        expected_vault_ratio_range: Option<PrecDecimalRange>,
    },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct LiquidityWithdrawerConfig {
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

        ensure_correct_vault(
            deps,
            vault_addr.to_string(),
            &self.lw_config.asset_data,
            &self.lw_config.lp_denom,
        )?;

        Ok(Config {
            input_addr,
            output_addr,
            vault_addr,
            lw_config: self.lw_config.clone(),
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

        if let Some(lw_config) = self.lw_config {
            config.lw_config = lw_config;
        }

        ensure_correct_vault(
            deps.as_ref(),
            config.vault_addr.to_string(),
            &config.lw_config.asset_data,
            &config.lw_config.lp_denom,
        )?;

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}
