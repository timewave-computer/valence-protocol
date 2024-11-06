use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;

use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::ValenceLibraryInterface;
use valence_osmosis_utils::utils::cl_utils::{query_cl_pool, TickRange};

#[cw_serde]
pub enum ActionMsgs {
    // provide liquidity at custom range
    ProvideLiquidityCustom {
        tick_range: TickRange,
        // default to 0 `token_min_amount` if not provided
        token_min_amount_0: Option<Uint128>,
        token_min_amount_1: Option<Uint128>,
    },
    // provide liquidity around the current tick
    ProvideLiquidityDefault {
        // bucket describes a tick range that spans between two ticks in the
        // interval that follows the configured tick spacing.
        // `bucket_amount` describes how many buckets around the currently
        // active bucket we want to cover (amplify the range) to each side.
        bucket_amount: Uint64,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetProcessor {},
    #[returns(Config)]
    GetLibraryConfig {},
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    pub pool_id: Uint64,
    pub pool_asset_1: String,
    pub pool_asset_2: String,
    pub global_tick_range: TickRange,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
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

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, Uint64), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        self.lp_config.global_tick_range.validate()?;

        Ok((input_addr, output_addr, self.lp_config.pool_id))
    }
}

/// Validated library configuration
#[cw_serde]
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
        let (input_addr, output_addr, pool_id) = self.do_validate(deps.api)?;
        let pool = query_cl_pool(&deps, pool_id.u64())?;

        let pool_assets = [pool.token0, pool.token1];

        let asset_1_found = pool_assets.contains(&self.lp_config.pool_asset_1);
        let asset_2_found = pool_assets.contains(&self.lp_config.pool_asset_2);

        ensure!(
            asset_1_found && asset_2_found,
            LibraryError::ExecutionError("Pool does not contain expected assets".to_string())
        );

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

        if let Some(cfg) = self.lp_config {
            config.lp_config = cfg;
        }

        Ok(())
    }
}
