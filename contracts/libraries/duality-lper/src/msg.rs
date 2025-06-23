use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;

use valence_library_utils::{
    error::LibraryError, liquidity_utils::AssetData, msg::LibraryConfigValidation,
    LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    /// Message to provide liquidity(deposit tokens).
    ProvideLiquidity {},
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    /// Address of the input account
    pub input_addr: LibraryAccountType,
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    /// Configuration for the liquidity provider
    /// This includes the pool address and asset data
    pub lp_config: LiquidityProviderConfig,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        lp_config: LiquidityProviderConfig,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            lp_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        api.addr_validate(&self.lp_config.pool_addr)?;

        Ok((input_addr, output_addr))
    }
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    /// Address of the pool we are going to provide liquidity for
    pub pool_addr: String,
    /// Denoms of both assets we are going to provide liquidity for
    pub asset_data: AssetData,
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub lp_config: LiquidityProviderConfig,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr) = self.do_validate(deps.api)?;

        ensure_correct_pool(
            self.lp_config.pool_addr.to_string(),
            &self.lp_config.asset_data,
            &deps,
        )?;

        Ok(Config {
            input_addr,
            output_addr,
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

        if let Some(lp_config) = self.lp_config {
            config.lp_config = lp_config;
            deps.api
                .addr_validate(config.lp_config.pool_addr.as_ref())?;
        }

        ensure_correct_pool(
            config.lp_config.pool_addr.to_string(),
            &config.lp_config.asset_data,
            &deps.as_ref(),
        )?;

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

fn ensure_correct_pool(
    pool_addr: String,
    assets: &AssetData,
    deps: &Deps,
) -> Result<(), LibraryError> {
    // Query the pool configuration
    let pool_config: valence_duality_utils::utils::PoolConfig = deps.querier.query_wasm_smart(
        pool_addr,
        &valence_duality_utils::msg::QueryMsg::GetConfig {},
    ).map_err(|e| LibraryError::ExecutionError(format!("Failed to query pool config: {}", e)))?;

    // Validate the denoms of the pool against the provided assets
    if pool_config.pair_data.token_0.denom != assets.asset1
        || pool_config.pair_data.token_1.denom != assets.asset2
    {
        return Err(LibraryError::ExecutionError(
            "Pool configuration does not match the provided asset data".to_string(),
        ));
    }

    Ok(())
}
