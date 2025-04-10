use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use valence_astroport_utils::PoolType;

use valence_library_utils::{
    error::LibraryError,
    liquidity_utils::{AssetData, DecimalRange},
    msg::LibraryConfigValidation,
    LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    ProvideDoubleSidedLiquidity {
        expected_pool_ratio_range: Option<DecimalRange>,
    },
    ProvideSingleSidedLiquidity {
        asset: String,
        limit: Option<Uint128>,
        expected_pool_ratio_range: Option<DecimalRange>,
    },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    pub pool_addr: String,
    pub lp_config: LiquidityProviderConfig,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        pool_addr: String,
        lp_config: LiquidityProviderConfig,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            pool_addr,
            lp_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let pool_addr = api.addr_validate(&self.pool_addr)?;

        Ok((input_addr, output_addr, pool_addr))
    }
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    /// Pool type, old Astroport pools use Cw20 lp tokens and new pools use native tokens, so we specify here what kind of token we are going to get.
    /// We also provide the PairType structure of the right Astroport version that we are going to use for each scenario
    pub pool_type: PoolType,
    /// Denoms of both native assets we are going to provide liquidity for
    pub asset_data: AssetData,
    /// Max spread used when swapping assets to provide single sided liquidity
    pub max_spread: Option<Decimal>,
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub pool_addr: Addr,
    pub lp_config: LiquidityProviderConfig,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, pool_addr) = self.do_validate(deps.api)?;

        ensure_correct_pool(
            self.pool_addr.to_string(),
            &self.lp_config.pool_type,
            &self.lp_config.asset_data,
            &deps,
        )?;

        Ok(Config {
            input_addr,
            output_addr,
            pool_addr,
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

        if let Some(pool_addr) = self.pool_addr {
            config.pool_addr = deps.api.addr_validate(&pool_addr)?;
        }

        if let Some(lp_config) = self.lp_config {
            config.lp_config = lp_config;
        }

        ensure_correct_pool(
            config.pool_addr.to_string(),
            &config.lp_config.pool_type,
            &config.lp_config.asset_data,
            &deps.as_ref(),
        )?;

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

fn ensure_correct_pool(
    pool_addr: String,
    pool_type: &PoolType,
    assets: &AssetData,
    deps: &Deps,
) -> Result<(), LibraryError> {
    match pool_type {
        PoolType::NativeLpToken(pair_type) => {
            let pool_response: valence_astroport_utils::astroport_native_lp_token::PairInfo =
                deps.querier.query_wasm_smart(
                    pool_addr,
                    &valence_astroport_utils::astroport_native_lp_token::PoolQueryMsg::Pair {},
                )?;

            if pool_response.pair_type != *pair_type {
                return Err(LibraryError::ConfigurationError(
                    "Pool type does not match the expected pair type".to_string(),
                ));
            }

            // Check that both assets in the pool are native and that they match our assets
            for (pool_asset, expected_asset) in pool_response
                .asset_infos
                .iter()
                .zip([&assets.asset1, &assets.asset2].iter())
            {
                match pool_asset {
                    valence_astroport_utils::astroport_native_lp_token::AssetInfo::Token { .. } => {
                        return Err(LibraryError::ConfigurationError(
                            "Pool asset is not a native token".to_string(),
                        ))
                    }
                    valence_astroport_utils::astroport_native_lp_token::AssetInfo::NativeToken { denom } => {
                        if denom != *expected_asset {
                            return Err(LibraryError::ConfigurationError(
                                "Pool asset does not match the expected asset".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        PoolType::Cw20LpToken(pair_type) => {
            let pool_response: valence_astroport_utils::astroport_cw20_lp_token::PairInfo =
                deps.querier.query_wasm_smart(
                    pool_addr,
                    &valence_astroport_utils::astroport_cw20_lp_token::PoolQueryMsg::Pair {},
                )?;

            if pool_response.pair_type != *pair_type {
                return Err(LibraryError::ConfigurationError(
                    "Pool type does not match the expected pair type".to_string(),
                ));
            }

            // Check that both assets in the pool are native and that they match our assets
            for (pool_asset, expected_asset) in pool_response
                .asset_infos
                .iter()
                .zip([&assets.asset1, &assets.asset2].iter())
            {
                match pool_asset {
                    valence_astroport_utils::astroport_cw20_lp_token::AssetInfo::Token {
                        ..
                    } => {
                        return Err(LibraryError::ConfigurationError(
                            "Pool asset is not a native token".to_string(),
                        ))
                    }
                    valence_astroport_utils::astroport_cw20_lp_token::AssetInfo::NativeToken {
                        denom,
                    } => {
                        if denom != *expected_asset {
                            return Err(LibraryError::ConfigurationError(
                                "Pool asset does not match the expected asset".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
