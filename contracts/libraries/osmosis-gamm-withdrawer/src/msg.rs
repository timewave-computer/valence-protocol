use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Deps, DepsMut, Uint64};
use cw_ownable::cw_ownable_query;

use osmosis_std::types::osmosis::poolmanager::v1beta1::PoolmanagerQuerier;
use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum ActionMsgs {
    WithdrawLiquidity {},
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct LiquidityWithdrawerConfig {
    pub pool_id: u64,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    pub lw_config: LiquidityWithdrawerConfig,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        lw_config: LiquidityWithdrawerConfig,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            lw_config,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, Uint64), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;

        Ok((input_addr, output_addr, self.lw_config.pool_id.into()))
    }
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub lw_config: LiquidityWithdrawerConfig,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, pool_id) = self.do_validate(deps.api)?;

        // just a sanity check to ensure the pool exists
        PoolmanagerQuerier::new(&deps.querier).pool(pool_id.u64())?;

        Ok(Config {
            input_addr,
            output_addr,
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

        if let Some(cfg) = self.lw_config {
            config.lw_config = cfg;
        }

        Ok(())
    }
}
