use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use valence_macros::{valence_service_query, ValenceServiceInterface};
use valence_service_utils::{
    error::ServiceError, msg::ServiceConfigValidation, ServiceAccountType,
};

use crate::rebalancer_custom::PID;

#[cw_serde]
pub enum ActionMsgs {
    StartRebalance {
        trustee: Option<String>,
        pid: PID,
        max_limit_bps: Option<u64>,
        min_balance: Uint128,
    },
    UpdateRebalancerConfig {
        trustee: Option<String>,
        pid: Option<PID>,
        max_limit_bps: Option<u64>,
    },
}

#[valence_service_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

/// Everything a service needs as a parameter to be instantiated goes into `ServiceConfig`
/// `ValenceServiceInterface` generates `ServiceConfigUpdate` is used in update method that allows to update the service configuration
/// `ServiceConfigUpdate` turns all fields <T> from `ServiceConfig` into Option<T>
///  
/// Fields that are Option<T>, will be generated as OptionUpdate<T>
/// If a field cannot or should not be updated, it should be annotated with #[skip_update]
#[cw_serde]
#[derive(ValenceServiceInterface)]
pub struct ServiceConfig {
    /// The "output" address of the account that will be registered to the rebalancer
    rebalancer_account: ServiceAccountType,
    /// The services manager in the reblaancer address
    /// this is used to send the register message to the rebalancer
    rebalancer_manager_addr: ServiceAccountType,
    /// List of denoms to add as targets
    denoms: Vec<String>,
    /// The main denom we rebalance into
    base_denom: String,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.rebalancer_account.to_addr(api)?;
        self.rebalancer_manager_addr.to_addr(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        Ok(Config {
            rebalancer_account: self.rebalancer_account.to_addr(deps.api)?,
            rebalancer_manager_addr: self.rebalancer_manager_addr.to_addr(deps.api)?,
            denoms: self.denoms.clone(),
            base_denom: self.base_denom.clone(),
        })
    }
}

impl ServiceConfigUpdate {
    /// Service developer must not forget to update config storage needed
    pub fn update_config(self, deps: DepsMut) -> Result<(), ServiceError> {
        let config: Config = valence_service_base::load_config(deps.storage)?;

        valence_service_base::save_config(deps.storage, &config)?;

        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    /// The "output" address of the account that will be registered to the rebalancer
    pub rebalancer_account: Addr,
    /// The services manager in the reblaancer address
    /// this is used to send the register message to the rebalancer
    pub rebalancer_manager_addr: Addr,
    /// List of denoms to add as targets
    pub denoms: Vec<String>,
    /// The main denom we rebalance into
    pub base_denom: String,
}