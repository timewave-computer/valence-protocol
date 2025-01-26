use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, StdError, StdResult, Storage, SubMsg, WasmMsg,
};

pub mod denoms {
    pub use cw_denom::{CheckedDenom, DenomError, UncheckedDenom};
}

pub mod error;
pub mod liquidity_utils;
pub mod msg;
pub mod raw_config;

#[cfg(feature = "testing")]
pub mod testing;

pub type Id = u64;

pub trait LibraryConfigUpdateTrait {
    fn update_raw(&self, storage: &mut dyn Storage) -> StdResult<()>;
}

#[cw_serde]
#[derive(Default)]
pub enum OptionUpdate<T> {
    #[default]
    None,
    Set(Option<T>),
}

/// An account type that is used in the library configs
/// It can either be an Id or Addr
/// The config that will be passed to the library must be of Addr veriant
#[cw_serde]
#[derive(Eq, PartialOrd, Ord)]
pub enum LibraryAccountType {
    #[serde(rename = "|library_account_addr|", alias = "library_account_addr")]
    Addr(String),
    #[serde(rename = "|account_id|", alias = "account_id")]
    AccountId(Id),
    #[serde(rename = "|library_id|", alias = "library_id")]
    LibraryId(Id),
}

impl From<&Addr> for LibraryAccountType {
    fn from(addr: &Addr) -> Self {
        LibraryAccountType::Addr(addr.to_string())
    }
}

impl From<&str> for LibraryAccountType {
    fn from(addr: &str) -> Self {
        if addr.starts_with("|account_id|:") {
            LibraryAccountType::AccountId(addr.trim_start_matches("|account_id|:").parse().unwrap())
        } else if addr.starts_with("|library_id|:") {
            LibraryAccountType::LibraryId(addr.trim_start_matches("|library_id|:").parse().unwrap())
        } else {
            LibraryAccountType::Addr(addr.to_owned())
        }
    }
}

pub trait GetId {
    fn get_account_id(&self) -> Id;
    fn get_library_id(&self) -> Id;
}

impl GetId for LibraryAccountType {
    fn get_account_id(&self) -> Id {
        match self {
            LibraryAccountType::Addr(_) => {
                panic!("LibraryAccountType is an address")
            }
            LibraryAccountType::AccountId(id) => *id,
            LibraryAccountType::LibraryId(_) => panic!("LibraryAccountType is a library id"),
        }
    }

    fn get_library_id(&self) -> Id {
        match self {
            LibraryAccountType::Addr(_) => {
                panic!("LibraryAccountType is an address")
            }
            LibraryAccountType::AccountId(_) => panic!("LibraryAccountType is a account id"),
            LibraryAccountType::LibraryId(id) => *id,
        }
    }
}

impl GetId for u64 {
    fn get_account_id(&self) -> Id {
        *self
    }

    fn get_library_id(&self) -> Id {
        *self
    }
}

impl GetId for &u64 {
    fn get_account_id(&self) -> Id {
        **self
    }

    fn get_library_id(&self) -> Id {
        **self
    }
}

impl GetId for u32 {
    fn get_account_id(&self) -> Id {
        (*self).into()
    }

    fn get_library_id(&self) -> Id {
        (*self).into()
    }
}

impl LibraryAccountType {
    pub fn to_string(&self) -> StdResult<String> {
        match self {
            LibraryAccountType::Addr(addr) => Ok(addr.to_string()),
            LibraryAccountType::AccountId(_) | LibraryAccountType::LibraryId(_) => Err(
                StdError::generic_err("LibraryAccountType must be an address"),
            ),
        }
    }

    pub fn to_addr(&self, api: &dyn cosmwasm_std::Api) -> StdResult<Addr> {
        match self {
            LibraryAccountType::Addr(addr) => api.addr_validate(addr),
            LibraryAccountType::AccountId(_) | LibraryAccountType::LibraryId(_) => Err(
                StdError::generic_err("LibraryAccountType must be an address"),
            ),
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

pub fn execute_submsgs_on_behalf_of(
    msgs: Vec<SubMsg>,
    payload: Option<String>,
    account: &Addr,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: account.to_string(),
        msg: to_json_binary(&valence_account_utils::msg::ExecuteMsg::ExecuteSubmsgs {
            msgs,
            payload,
        })?,
        funds: vec![],
    }))
}
