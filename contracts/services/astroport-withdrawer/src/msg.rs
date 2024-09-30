use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Api, Decimal, Deps, DepsMut};
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
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn Api) -> Result<(), ServiceError>;
    fn validate(&self, deps: Deps) -> Result<T, ServiceError>;
}

pub trait ServiceConfigInterface<T> {
    /// T is the config type
    fn is_diff(&self, other: &T) -> bool;
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
    WithdrawLiquidity {},
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
    pub withdrawer_config: LiquidityWithdrawerConfig,
}

impl ServiceConfig {
    pub fn new(
        input_addr: String,
        output_addr: String,
        pool_addr: String,
        withdrawer_config: LiquidityWithdrawerConfig,
    ) -> Self {
        ServiceConfig {
            input_addr,
            output_addr,
            pool_addr,
            withdrawer_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), ServiceError> {
        let input_addr = api.addr_validate(&self.input_addr)?;
        let output_addr = api.addr_validate(&self.output_addr)?;
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

impl ServiceConfigInterface<ServiceConfig> for ServiceConfig {
    /// This function is used to see if 2 configs are different
    fn is_diff(&self, other: &ServiceConfig) -> bool {
        !self.eq(other)
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

        if let Some(withdrawer_config) = self.withdrawer_config {
            config.withdrawer_config = withdrawer_config;
        }
        Ok(())
    }
}
