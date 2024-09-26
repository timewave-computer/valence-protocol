use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{ensure, Addr, Decimal, Deps};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

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
pub struct ServiceConfig {
    pub input_addr: String,
    pub output_addr: String,
    pub pool_addr: String,
    pub withdrawer_config: LiquidityWithdrawerConfig,
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
    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let input_addr = deps.api.addr_validate(&self.input_addr)?;
        let output_addr = deps.api.addr_validate(&self.output_addr)?;
        let pool_addr = deps.api.addr_validate(&self.pool_addr)?;

        Ok(Config {
            input_addr,
            output_addr,
            pool_addr,
            withdrawer_config: self.withdrawer_config.clone(),
        })
    }
}
