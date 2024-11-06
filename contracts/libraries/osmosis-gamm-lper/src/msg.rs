use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use osmosis_std::types::osmosis::poolmanager::v1beta1::PoolmanagerQuerier;
use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};
use valence_osmosis_utils::utils::{gamm_utils::ValenceLiquidPooler, DecimalRange};

#[cw_serde]
pub enum ActionMsgs {
    ProvideDoubleSidedLiquidity {
        expected_spot_price: Option<DecimalRange>,
    },
    ProvideSingleSidedLiquidity {
        expected_spot_price: Option<DecimalRange>,
        asset: String,
        limit: Uint128,
    },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct LiquidityProviderConfig {
    pub pool_id: u64,
    pub pool_asset_1: String,
    pub pool_asset_2: String,
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

        Ok((input_addr, output_addr, self.lp_config.pool_id.into()))
    }
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
        let (input_addr, output_addr, _pool_id) = self.do_validate(deps.api)?;

        let pm_querier = PoolmanagerQuerier::new(&deps.querier);
        let pool = pm_querier.query_pool_config(self.lp_config.pool_id)?;

        // perform soft pool validation by asserting that the lp config assets
        // are all present in the pool
        let (mut asset_1_found, mut asset_2_found) = (false, false);
        for pool_asset in pool.pool_assets {
            if let Some(asset) = pool_asset.token {
                if self.lp_config.pool_asset_1 == asset.denom {
                    asset_1_found = true;
                }
                if self.lp_config.pool_asset_2 == asset.denom {
                    asset_2_found = true;
                }
            }
        }

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
