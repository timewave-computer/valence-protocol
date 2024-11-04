use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Deps, DepsMut, Uint64};
use cw_ownable::cw_ownable_query;

use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType,
};

#[cw_serde]
pub enum ActionMsgs {
    WithdrawLiquidity {},
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct LiquidityWithdrawerConfig {
    pub pool_id: u64,
}

#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub output_addr: ServiceAccountType,
    pub lp_config: LiquidityWithdrawerConfig,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        output_addr: impl Into<ServiceAccountType>,
        lp_config: LiquidityWithdrawerConfig,
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
    pub lp_config: LiquidityWithdrawerConfig,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let (input_addr, output_addr, _pool_id) = self.do_validate(deps.api)?;

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
