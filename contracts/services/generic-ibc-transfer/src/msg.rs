use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use getset::{Getters, Setters};
use valence_macros::OptionalStruct;
use valence_service_utils::{
    denoms::{CheckedDenom, UncheckedDenom},
    error::ServiceError,
    msg::ServiceConfigValidation,
    ServiceAccountType, ServiceConfigInterface,
};

#[cw_serde]
pub enum ActionMsgs {
    IbcTransfer {},
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
    pub amount: IbcTransferAmount,
    pub memo: String,
    pub remote_chain_info: RemoteChainInfo,
}

#[cw_serde]
pub enum IbcTransferAmount {
    FullAmount,
    FixedAmount(Uint128),
}

#[cw_serde]
pub struct RemoteChainInfo {
    pub channel_id: String,
    pub ibc_transfer_timeout: Option<Uint64>,
}

impl ServiceConfig {
    pub fn new(
        input_addr: ServiceAccountType,
        output_addr: String,
        denom: UncheckedDenom,
        amount: IbcTransferAmount,
        memo: String,
        remote_chain_info: RemoteChainInfo,
    ) -> Self {
        Self {
            input_addr,
            output_addr,
            denom,
            amount,
            memo,
            remote_chain_info,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, ServiceError> {
        let input_addr = self.input_addr.to_addr(api)?;

        match self.amount {
            IbcTransferAmount::FullAmount => {}
            IbcTransferAmount::FixedAmount(amount) => {
                if amount.is_zero() {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid IBC transfer config: amount cannot be zero.".to_string(),
                    ));
                }
            }
        }

        if self.remote_chain_info.channel_id.is_empty() {
            return Err(ServiceError::ConfigurationError(
                "Invalid IBC transfer config: remote_chain_info's channel_id cannot be empty."
                    .to_string(),
            ));
        }

        if let Some(timeout) = self.remote_chain_info.ibc_transfer_timeout {
            if timeout.is_zero() {
                return Err(ServiceError::ConfigurationError(
                    "Invalid IBC transfer config: remote_chain_info's ibc_transfer_timeout cannot be zero.".to_string(),
                ));
            }
        }

        Ok(input_addr)
    }
}

impl ServiceConfigValidation<Config> for ServiceConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), ServiceError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, ServiceError> {
        let input_addr = self.do_validate(deps.api)?;
        Ok(Config {
            input_addr,
            // Can't validate output address as it's on another chain
            output_addr: Addr::unchecked(self.output_addr.clone()),
            denom: self
                .denom
                .clone()
                .into_checked(deps)
                .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?,
            amount: self.amount.clone(),
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
    pub fn update_config(self, deps: &DepsMut, config: &mut Config) -> Result<(), ServiceError> {
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        if let Some(output_addr) = self.output_addr {
            config.output_addr = Addr::unchecked(output_addr);
        }

        if let Some(denom) = self.denom {
            config.denom = denom
                .clone()
                .into_checked(deps.as_ref())
                .map_err(|err| ServiceError::ConfigurationError(err.to_string()))?;
        }

        if let Some(amount) = self.amount {
            if let IbcTransferAmount::FixedAmount(amount) = &amount {
                if amount.is_zero() {
                    return Err(ServiceError::ConfigurationError(
                        "Invalid IBC transfer config: amount cannot be zero.".to_string(),
                    ));
                }
            }
            config.amount = amount;
        }

        if let Some(memo) = self.memo {
            config.memo = memo;
        }

        if let Some(remote_chain_info) = self.remote_chain_info {
            config.remote_chain_info = remote_chain_info;
        }

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
    amount: IbcTransferAmount,
    #[getset(get = "pub", set)]
    memo: String,
    #[getset(get = "pub", set)]
    remote_chain_info: RemoteChainInfo,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        output_addr: Addr,
        denom: CheckedDenom,
        amount: IbcTransferAmount,
        memo: String,
        remote_chain_info: RemoteChainInfo,
    ) -> Self {
        Config {
            input_addr,
            output_addr,
            denom,
            amount,
            memo,
            remote_chain_info,
        }
    }
}
