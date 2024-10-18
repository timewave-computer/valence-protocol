use std::str::FromStr;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Decimal, Deps, DepsMut, Empty, StdError, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use osmosis_std::types::osmosis::{gamm::v1beta1::Pool, poolmanager::v1beta1::PoolmanagerQuerier};
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_osmosis_utils::utils::DecimalRange;
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType,
};

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

#[valence_service_query]
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
#[derive(ValenceServiceInterface)]
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

impl ServiceConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), ServiceError> {
        let mut config: Config = valence_service_base::load_config(deps.storage)?;

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
