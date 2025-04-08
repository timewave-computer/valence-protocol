use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdResult, Storage, SubMsg, WasmMsg};

pub mod denoms {
    pub use cw_denom::{CheckedDenom, DenomError, UncheckedDenom};
}

pub mod error;
pub mod ica;
pub mod library_account_type;
pub mod liquidity_utils;
pub mod msg;
pub mod raw_config;

#[cfg(feature = "testing")]
pub mod testing;

pub type Id = u64;

pub use library_account_type::LibraryAccountType;

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
