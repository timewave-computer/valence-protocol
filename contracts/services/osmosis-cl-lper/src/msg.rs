use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Int64, StdError, Uint128, Uint64};
use cw_ownable::cw_ownable_query;

use osmosis_std::types::osmosis::{
    concentratedliquidity::v1beta1::Pool, poolmanager::v1beta1::PoolmanagerQuerier,
};
use valence_macros::ValenceServiceInterface;
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType,
};

#[cw_serde]
pub enum ActionMsgs {
    // provide liquidity at custom
    ProvideLiquidityCustom {
        lower_tick: Int64,
        upper_tick: Int64,
        // default to 0 `token_min_amount` if not provided
        token_min_amount_0: Option<Uint128>,
        token_min_amount_1: Option<Uint128>,
    },
    ProvideLiquidityDefault {},
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
pub struct LiquidityProviderConfig {
    pub pool_id: Uint64,
    pub pool_asset_1: String,
    pub pool_asset_2: String,
    pub global_tick_min: Int64,
    pub global_tick_max: Int64,
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

        Ok((input_addr, output_addr, self.lp_config.pool_id))
    }
}

/// Validated service configuration
#[cw_serde]
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
        let (input_addr, output_addr, pool_id) = self.do_validate(deps.api)?;

        let pm_querier = PoolmanagerQuerier::new(&deps.querier);
        let pool_response = pm_querier.pool(pool_id.u64())?;

        let pool_proto = pool_response
            .pool
            .ok_or_else(|| StdError::generic_err("pool not found"))?;

        let pool: Pool = pool_proto
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode CL pool proto"))?;

        // perform soft pool validation by asserting that the lp config assets
        // are all present in the pool
        let (mut asset_1_found, mut asset_2_found) = (false, false);
        for pool_asset in [pool.token0, pool.token1] {
            if self.lp_config.pool_asset_1 == pool_asset {
                asset_1_found = true;
            }
            if self.lp_config.pool_asset_2 == pool_asset {
                asset_2_found = true;
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
