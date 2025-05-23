use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Deps, DepsMut, Uint256};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

use crate::state::WithdrawalObligation;

#[cw_serde]
/// Validated library configuration
pub struct Config {
    /// settlement input account which we tap into in order
    /// to settle the obligations
    pub input_addr: Addr,
    /// authorized strategist
    pub strategist: Addr,
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    /// settlement input account which we tap into in order
    /// to settle the obligations
    pub input_addr: LibraryAccountType,
    /// authorized strategist
    pub strategist: String,
}

#[cw_serde]
pub enum FunctionMsgs {
    /// validates and enqueues a new withdrawal obligation
    RegisterObligation {
        /// where the payout is to be routed
        recipient: String,
        /// what is owed to the recipient
        payout_coins: Vec<Coin>,
        /// some unique identifier for the request
        id: Uint256,
    },
    /// settles the oldest withdrawal obligation
    SettleNextObligation {},
}

impl LibraryConfig {
    pub fn new(input_addr: impl Into<LibraryAccountType>, strategist: String) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            strategist,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr), LibraryError> {
        // validate the input account
        let input_addr = self.input_addr.to_addr(api)?;
        // validate the strategist address
        let strategist_addr = api.addr_validate(&self.strategist)?;

        Ok((input_addr, strategist_addr))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, strategist) = self.do_validate(deps.api)?;
        Ok(Config {
            input_addr,
            strategist,
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        if let Some(strategist_addr) = self.strategist {
            config.strategist = deps.api.addr_validate(&strategist_addr)?;
        }

        valence_library_base::save_config(deps.storage, &config)?;

        Ok(())
    }
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// returns the total number of obligations in the queue
    #[returns(QueueInfoResponse)]
    QueueInfo {},
    /// returns a list of obligations in the queue starting from the given index
    #[returns(ObligationsResponse)]
    Obligations {
        /// starting index
        from: Option<u64>,
        /// end index
        to: Option<u64>,
    },
}

#[cw_serde]
pub struct QueueInfoResponse {
    /// total number of obligations in the queue
    pub count: u64,
    /// starting index of the queue (pagination)
    pub start_index: u64,
    /// ending index of the queue (pagination)
    pub end_index: u64,
}

#[cw_serde]
pub struct ObligationsResponse {
    pub obligations: Vec<WithdrawalObligation>,
}
