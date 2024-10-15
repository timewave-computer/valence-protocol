use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType,
};

#[cw_serde]
pub enum ActionsMsgs {
    WithdrawLiquidity {},
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub output_addr: ServiceAccountType,
    pub pool_addr: String,
    pub withdrawer_config: LiquidityWithdrawerConfig,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        output_addr: impl Into<ServiceAccountType>,
        pool_addr: String,
        withdrawer_config: LiquidityWithdrawerConfig,
    ) -> Self {
        ServiceConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            pool_addr,
            withdrawer_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), ServiceError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let pool_addr = api.addr_validate(&self.pool_addr)?;

        Ok((input_addr, output_addr, pool_addr))
    }
}

#[cw_serde]
pub struct LiquidityWithdrawerConfig {
    /// Pool type, old Astroport pools use Cw20 lp tokens and new pools use native tokens, so we specify here what kind of token we are going to get.
    /// We also provide the PairType structure of the right Astroport version that we are going to use for each scenario
    pub pool_type: PoolType,
}

#[cw_serde]
pub enum PoolType {
    NativeLpToken,
    Cw20LpToken,
}

#[cw_serde]
/// Validated service configuration
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub pool_addr: Addr,
    pub withdrawer_config: LiquidityWithdrawerConfig,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let (input_addr, output_addr, pool_addr) = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            pool_addr,
            withdrawer_config: self.withdrawer_config.clone(),
        })
    }
}

impl ServiceConfigUpdate {
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_addr(deps.api)?;
        }

        if let Some(pool_addr) = self.pool_addr {
            config.pool_addr = deps.api.addr_validate(&pool_addr)?;
        }

        if let Some(withdrawer_config) = self.withdrawer_config {
            config.withdrawer_config = withdrawer_config;
        }
        Ok(())
    }
}
