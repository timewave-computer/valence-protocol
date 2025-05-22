use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Deps, DepsMut, Uint256};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub enum FunctionMsgs {
    /// validates and enqueues a new withdrawal obligation
    RegisterObligation(WithdrawalObligation),
    /// settles the oldest withdrawal obligation
    SettleNextObligation {},
}

/// unsettled liability sitting in the clearing queue
#[cw_serde]
pub struct WithdrawalObligation {
    pub recipient: String,       // where the payout is to be routed
    pub payout_coins: Vec<Coin>, // what is owed to the recipient
    pub id: Uint256,             // some unique identifier for the request
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub input_addr: LibraryAccountType,
    pub strategist: String,
}

impl LibraryConfig {
    pub fn new(input_addr: impl Into<LibraryAccountType>, strategist: String) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            strategist,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;

        let strategist_addr = api.addr_validate(&self.strategist)?;

        Ok((input_addr, strategist_addr))
    }
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub input_addr: Addr,
    pub strategist: Addr,
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
