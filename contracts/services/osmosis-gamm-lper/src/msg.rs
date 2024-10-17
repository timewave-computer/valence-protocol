use std::str::FromStr;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    ensure, Addr, Decimal, Deps, DepsMut, Empty, StdError, StdResult, Uint128, Uint64,
};
use cw_ownable::cw_ownable_query;
use osmosis_std::types::osmosis::{gamm::v1beta1::Pool, poolmanager::v1beta1::PoolmanagerQuerier};

use valence_macros::OptionalStruct;
use valence_osmosis_utils::utils::{DecimalRange, LiquidityProviderConfig};
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType, ServiceConfigInterface,
};
#[cw_serde]
pub enum ActionsMsgs {
    ProvideDoubleSidedLiquidity {
        expected_spot_price: Option<DecimalRange>,
    },
    ProvideSingleSidedLiquidity {
        expected_spot_price: Option<DecimalRange>,
        asset: String,
        limit: Uint128,
    },
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

pub trait ValenceLiquidPooler {
    fn query_spot_price(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Decimal>;
    fn query_pool_config(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Pool>;
}

impl ValenceLiquidPooler for PoolmanagerQuerier<'_, Empty> {
    fn query_spot_price(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Decimal> {
        let spot_price_response = self.spot_price(
            lp_config.pool_id,
            lp_config.pool_asset_1.to_string(),
            lp_config.pool_asset_2.to_string(),
        )?;

        let pool_ratio = Decimal::from_str(&spot_price_response.spot_price)?;

        Ok(pool_ratio)
    }

    fn query_pool_config(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Pool> {
        let pool_response = self.pool(lp_config.pool_id)?;
        let pool: Pool = pool_response
            .pool
            .ok_or_else(|| StdError::generic_err("failed to get pool"))?
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode proto"))?;

        Ok(pool)
    }
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub output_addr: ServiceAccountType,
    pub lp_config: LiquidityProviderConfig,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        output_addr: impl Into<ServiceAccountType>,
        lp_config: LiquidityProviderConfig,
    ) -> Self {
        ServiceConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            lp_config,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, Uint64), ServiceError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;

        Ok((input_addr, output_addr, self.lp_config.pool_id.into()))
    }
}

impl ServiceConfigInterface<ServiceConfig> for ServiceConfig {
    /// This function is used to see if 2 configs are different
    fn is_diff(&self, other: &ServiceConfig) -> bool {
        !self.eq(other)
    }
}

#[cw_serde]
/// Validated service configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub lp_config: LiquidityProviderConfig,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let (input_addr, output_addr, _pool_id) = self.do_validate(deps.api)?;

        let pm_querier = PoolmanagerQuerier::new(&deps.querier);
        let pool = pm_querier.query_pool_config(&self.lp_config)?;

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
            ServiceError::ExecutionError("Pool does not contain expected assets".to_string())
        );

        Ok(Config {
            input_addr,
            output_addr,
            lp_config: self.lp_config.clone(),
        })
    }
}

impl OptionalServiceConfig {
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
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