use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, DepsMut, Uint64};
use valence_macros::OptionalStruct;
use valence_osmosis_utils::utils::OsmosisPoolType;
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType, ServiceConfigInterface,
};

use crate::msg::LiquidityProviderConfig;

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub output_addr: ServiceAccountType,
    pub lp_config: LiquidityProviderConfig,
    pub pool_type: OsmosisPoolType,
}

impl ServiceConfig {
    pub fn new(
        input_addr: impl Into<ServiceAccountType>,
        output_addr: impl Into<ServiceAccountType>,
        lp_config: LiquidityProviderConfig,
        pool_type: OsmosisPoolType,
    ) -> Self {
        ServiceConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            lp_config,
            pool_type,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, Uint64), ServiceError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;

        // TODO: validate pool_id?

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
