use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdError, StdResult, WasmMsg};
pub mod denoms {
    pub use cw_denom::{CheckedDenom, DenomError, UncheckedDenom};
}

pub mod error;
pub mod msg;

#[cfg(feature = "testing")]
pub mod testing;

pub type Id = u64;

pub trait ServiceConfigInterface<T> {
    /// T is the config type
    fn is_diff(&self, other: &T) -> bool;
}

/// An account type that is used in the service configs
/// It can either be an Id or Addr
/// The config that will be passed to the service must be of Addr veriant
#[cw_serde]
#[derive(Eq, PartialOrd, Ord)]
pub enum ServiceAccountType {
    #[serde(rename = "|account_addr|", alias = "account_addr")]
    AccountAddr(String),
    #[serde(rename = "|account_id|", alias = "account_id")]
    AccountId(Id),
}

impl From<&Addr> for ServiceAccountType {
    fn from(addr: &Addr) -> Self {
        ServiceAccountType::AccountAddr(addr.to_string())
    }
}

impl From<&str> for ServiceAccountType {
    fn from(addr: &str) -> Self {
        if addr.starts_with("|account_id|:") {
            ServiceAccountType::AccountId(addr.trim_start_matches("|account_id|:").parse().unwrap())
        } else {
            ServiceAccountType::AccountAddr(addr.to_owned())
        }
    }
}

impl ServiceAccountType {
    pub fn to_string(&self) -> StdResult<String> {
        match self {
            ServiceAccountType::AccountAddr(addr) => Ok(addr.to_string()),
            ServiceAccountType::AccountId(_) => {
                Err(StdError::generic_err("Account type is not an address"))
            }
        }
    }

    pub fn to_addr(&self, api: &dyn cosmwasm_std::Api) -> StdResult<Addr> {
        match self {
            ServiceAccountType::AccountAddr(addr) => api.addr_validate(addr),
            ServiceAccountType::AccountId(_) => {
                Err(StdError::generic_err("Account type is not an address"))
            }
        }
    }
}

// This is a helper function to execute a CosmosMsg on behalf of an account
pub fn execute_on_behalf_of(msgs: Vec<CosmosMsg>, account: &Addr) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: account.to_string(),
        msg: to_json_binary(&valence_account_utils::msg::ExecuteMsg::ExecuteMsg { msgs })?,
        funds: vec![],
    }))
}
