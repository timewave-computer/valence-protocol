use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use valence_macros::OptionalStruct;
use valence_service_utils::{error::ServiceError, msg::ServiceConfigValidation};

#[cw_serde]
pub enum ActionsMsgs {
    ProvideDoubleSidedLiquidity {},
    ProvideSingleSidedLiquidity {},
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetProcessor {},
    #[returns(Config)]
    GetServiceConfig {},
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    pub input_addr: String,
    pub output_addr: String,
    pub pool_addr: String,
    pub lp_config: LiquidityProviderConfig,
}

#[cw_serde]
pub struct LiquidityProviderConfig {
    /// LP token type, old Astroport pools use Cw20 lp tokens and new pools use native tokens, so we specify here what kind of token we are going to get.
    /// Also useful to know which version of Astroport we are going to interact with
    pub lp_token_type: LpTokenType,
    /// Denoms of both native assets we are going to provide liquidity for
    pub asset_data: AssetData,
    /// Amounts of both tokens we consider OK to single-side lp
    pub single_side_lp_limits: SingleSideLpLimits,
    pub slippage_tolerance: Option<Decimal>,
    /// Config for the pool price expectations upon instantiation
    pub pool_price_config: PoolPriceConfig,
}

#[cw_serde]
pub enum LpTokenType {
    Native,
    Cw20,
}

#[cw_serde]
pub struct AssetData {
    /// Denom of the first asset
    pub asset1: String,
    /// Denom of the second asset
    pub asset2: String,
}

#[cw_serde]
pub struct SingleSideLpLimits {
    pub asset1_limit: Uint128,
    pub asset2_limit: Uint128,
}

#[cw_serde]
pub struct PoolPriceConfig {
    pub expected_spot_price: Decimal,
    pub acceptable_price_spread: Decimal,
}

#[cw_serde]
/// Validated service configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub pool_addr: Addr,
    pub lp_config: LiquidityProviderConfig,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let input_addr = deps.api.addr_validate(&self.input_addr)?;
        let output_addr = deps.api.addr_validate(&self.output_addr)?;
        let pool_addr = deps.api.addr_validate(&self.pool_addr)?;

        ensure_asset_uniqueness(&self.lp_config.asset_data)?;

        Ok(Config {
            input_addr,
            output_addr,
            pool_addr,
            lp_config: self.lp_config.clone(),
        })
    }
}

impl OptionalServiceConfig {
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
        if let Some(input_addr) = self.input_addr {
            config.input_addr = deps.api.addr_validate(&input_addr)?;
        }

        if let Some(output_addr) = self.output_addr {
            config.output_addr = deps.api.addr_validate(&output_addr)?;
        }

        if let Some(pool_addr) = self.pool_addr {
            config.pool_addr = deps.api.addr_validate(&pool_addr)?;
        }

        if let Some(lp_config) = self.lp_config {
            ensure_asset_uniqueness(&lp_config.asset_data)?;
            config.lp_config = lp_config;
        }

        Ok(())
    }
}

fn ensure_asset_uniqueness(asset_data: &AssetData) -> Result<(), ServiceError> {
    if asset_data.asset1 == asset_data.asset2 {
        return Err(ServiceError::ConfigurationError(
            "Asset1 and Asset2 cannot be the same".to_string(),
        ));
    }
    Ok(())
}
