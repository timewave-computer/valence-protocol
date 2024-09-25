use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Decimal, Deps, DepsMut, Uint128};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use valence_macros::OptionalStruct;

use crate::error::ServiceError;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub processor: String,
    pub config: ServiceConfig,
}

pub trait ServiceConfigValidation<T> {
    fn validate(&self, deps: Deps) -> Result<T, ServiceError>;
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    ProcessAction(ActionsMsgs),
    UpdateConfig { new_config: ServiceConfig },
    UpdateProcessor { processor: String },
}

#[cw_serde]
pub enum ActionsMsgs {
    ProvideDoubleSidedLiquidity {
        expected_pool_ratio_range: Option<DecimalRange>,
    },
    ProvideSingleSidedLiquidity {
        asset: String,
        limit: Option<Uint128>,
        expected_pool_ratio_range: Option<DecimalRange>,
    },
}

#[cw_serde]
pub struct DecimalRange {
    min: Decimal,
    max: Decimal,
}

impl DecimalRange {
    pub fn is_within_range(&self, value: Decimal) -> Result<(), ServiceError> {
        ensure!(
            value >= self.min && value <= self.max,
            ServiceError::ExecutionError("Value is not within the expected range".to_string())
        );
        Ok(())
    }
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
    /// Pool type, old Astroport pools use Cw20 lp tokens and new pools use native tokens, so we specify here what kind of token we are going to get.
    /// We also provide the PairType structure of the right Astroport version that we are going to use for each scenario
    pub pool_type: PoolType,
    /// Denoms of both native assets we are going to provide liquidity for
    pub asset_data: AssetData,
    /// Slippage tolerance when providing liquidity
    pub slippage_tolerance: Option<Decimal>,
}

#[cw_serde]
pub enum PoolType {
    NativeLpToken(astroport::factory::PairType),
    Cw20LpToken(astroport_cw20_lp_token::factory::PairType),
}

#[cw_serde]
pub struct AssetData {
    /// Denom of the first asset
    pub asset1: String,
    /// Denom of the second asset
    pub asset2: String,
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
        ensure_correct_pool_type(self.pool_addr.to_string(), &self.lp_config.pool_type, &deps)?;

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

        ensure_correct_pool_type(
            config.pool_addr.to_string(),
            &config.lp_config.pool_type,
            &deps.as_ref(),
        )?;

        Ok(())
    }
}

fn ensure_correct_pool_type(
    pool_addr: String,
    pool_type: &PoolType,
    deps: &Deps,
) -> Result<(), ServiceError> {
    match pool_type {
        PoolType::NativeLpToken(pair_type) => {
            let pool_response: astroport::asset::PairInfo = deps
                .querier
                .query_wasm_smart(pool_addr, &astroport::pair::QueryMsg::Pair {})?;

            if pool_response.pair_type != *pair_type {
                return Err(ServiceError::ConfigurationError(
                    "Pool type does not match the expected pair type".to_string(),
                ));
            }
        }
        PoolType::Cw20LpToken(pair_type) => {
            let pool_response: astroport_cw20_lp_token::asset::PairInfo = deps
                .querier
                .query_wasm_smart(pool_addr, &astroport_cw20_lp_token::pair::QueryMsg::Pair {})?;

            if pool_response.pair_type != *pair_type {
                return Err(ServiceError::ConfigurationError(
                    "Pool type does not match the expected pair type".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn ensure_asset_uniqueness(asset_data: &AssetData) -> Result<(), ServiceError> {
    if asset_data.asset1 == asset_data.asset2 {
        return Err(ServiceError::ConfigurationError(
            "Asset1 and Asset2 cannot be the same".to_string(),
        ));
    }
    Ok(())
}
