use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;

use valence_library_utils::{
    error::LibraryError, liquidity_utils::AssetData, msg::LibraryConfigValidation,
    LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};
use valence_osmosis_utils::utils::cl_utils::TickRange;
use valence_vortex_utils::msg::CreatePositionMsg;
#[cw_serde]
pub enum FunctionMsgs {
    /// Message to provide liquidity(deposit tokens).
    ProvideLiquidity {
        tick_range: TickRange,
        principal_token_min_amount: Uint128,
        counterparty_token_min_amount: Uint128,
    },
    WithdrawLiquidity {},
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the vortex contract address
    #[returns(String)]
    GetVortexAddress {},
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    /// Address of the input account
    pub input_addr: LibraryAccountType,
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    /// Address of the second output account
    pub output_addr_2: LibraryAccountType,
    /// Configuration for the liquidity provider
    /// This includes the pool address and asset data
    pub lp_config: LiquidityProviderConfig,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        output_addr_2: impl Into<LibraryAccountType>,
        lp_config: LiquidityProviderConfig,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            output_addr_2: output_addr_2.into(),
            lp_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let output_addr_2 = self.output_addr_2.to_addr(api)?;

        Ok((input_addr, output_addr, output_addr_2))
    }
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    /// Code of the vortex contract we are going to instantiate
    pub vortex_code: u64,
    /// Label for the contract instantiation
    pub label: String,
    /// Id of the pool we are going to provide liquidity for
    pub pool_id: u64,
    /// Duration of the round in seconds
    pub round_duration: u64,
    /// Duration of the auction in seconds
    pub auction_duration: u64,
    /// Denoms of both assets we are going to provide liquidity for
    pub asset_data: AssetData,
    /// Whether the principal token is first in the pool
    pub principal_first: bool,
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub output_addr_2: Addr,
    pub lp_config: LiquidityProviderConfig,
}

#[cw_serde]
pub struct ReplyPayload {
    pub config: Config,
    pub create_position_msg: CreatePositionMsg,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, output_addr_2) = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            output_addr_2,
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

        if let Some(output_addr_2) = self.output_addr_2 {
            config.output_addr_2 = output_addr_2.to_addr(deps.api)?;
        }

        if let Some(lp_config) = self.lp_config {
            config.lp_config = lp_config;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}
