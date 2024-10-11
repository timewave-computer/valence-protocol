use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use getset::{Getters, Setters};
use neutron_sdk::bindings::query::NeutronQuery;
use valence_macros::OptionalStruct;
use valence_service_utils::{
    denoms::{CheckedDenom, UncheckedDenom},
    error::ServiceError,
    msg::ServiceConfigValidation,
    ServiceAccountType, ServiceConfigInterface,
};

#[cw_serde]
pub enum ActionsMsgs {
    IbcTransfer {},
    RefundDust {},
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {
    /// Query to get the processor address.
    #[returns(Addr)]
    GetProcessor {},
    /// Query to get the service configuration.
    #[returns(Config)]
    GetServiceConfig {},
}

#[cw_serde]
#[derive(OptionalStruct)]
pub struct ServiceConfig {
    pub input_addr: ServiceAccountType,
    pub output_addr: String,
    pub denom: UncheckedDenom,
    pub amount: Uint128,
    pub memo: String,
    pub remote_chain_info: RemoteChainInfo,
}

#[cw_serde]
pub struct RemoteChainInfo {
    pub channel_id: String,
    pub port_id: Option<String>,
    pub denom: String,
    pub ibc_transfer_timeout: Option<Uint64>,
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, _api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        Ok(Config {
            input_addr: self.input_addr.to_addr(deps.api)?,
            // Can't validate output address as it's on another chain
            output_addr: Addr::unchecked(self.output_addr.clone()),
            denom: self
                .denom
                .clone()
                .into_checked(deps)
                .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?,
            amount: self.amount,
            memo: self.memo.clone(),
            remote_chain_info: self.remote_chain_info.clone(),
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
    pub fn update_config(
        self,
        _deps: &DepsMut<NeutronQuery>,
        _config: &mut Config,
    ) -> Result<(), ServiceError> {
        Ok(())
    }
}

#[cw_serde]
#[derive(Getters, Setters)]
pub struct Config {
    #[getset(get = "pub", set)]
    input_addr: Addr,
    #[getset(get = "pub", set)]
    output_addr: Addr,
    #[getset(get = "pub", set)]
    denom: CheckedDenom,
    #[getset(get = "pub", set)]
    amount: Uint128,
    #[getset(get = "pub", set)]
    memo: String,
    #[getset(get = "pub", set)]
    remote_chain_info: RemoteChainInfo,
}
