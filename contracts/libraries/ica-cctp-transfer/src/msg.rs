use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    Transfer {},
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
/// Enum representing the different query messages that can be sent.
pub enum QueryMsg {}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    // Address of the input account (Valence interchain account)
    pub input_addr: LibraryAccountType,
    // Amount that is going to be transferred
    pub amount: Uint128,
    // Denom that is going to be transferred
    pub denom: String,
    // Destination domain id
    pub destination_domain_id: u32,
    // Address of the recipient account on the destination domain
    pub mint_recipient: Binary,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        amount: Uint128,
        denom: String,
        destination_domain_id: u32,
        mint_recipient: Binary,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            amount,
            denom,
            destination_domain_id,
            mint_recipient,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<Addr, LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        if self.amount.is_zero() {
            return Err(LibraryError::ConfigurationError(
                "Invalid transfer config: amount cannot be zero.".to_string(),
            ));
        }

        Ok(input_addr)
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let input_addr = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            amount: self.amount,
            denom: self.denom.clone(),
            destination_domain_id: self.destination_domain_id,
            mint_recipient: self.mint_recipient.clone(),
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        // First update input_addr (if needed)
        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        // Next update the amount (if needed)
        if let Some(amount) = self.amount {
            if amount.is_zero() {
                return Err(LibraryError::ConfigurationError(
                    "Invalid IBC transfer config: amount cannot be zero.".to_string(),
                ));
            }
            config.amount = amount;
        }

        // Next update the denom (if needed)
        if let Some(denom) = self.denom {
            config.denom = denom;
        }

        // Next update the destination domain id (if needed)
        if let Some(destination_domain_id) = self.destination_domain_id {
            config.destination_domain_id = destination_domain_id;
        }

        // Next update the mint recipient (if needed)
        if let Some(mint_recipient) = self.mint_recipient {
            config.mint_recipient = mint_recipient;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub amount: Uint128,
    pub denom: String,
    pub destination_domain_id: u32,
    pub mint_recipient: Binary,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        amount: Uint128,
        denom: String,
        destination_domain_id: u32,
        mint_recipient: Binary,
    ) -> Self {
        Config {
            input_addr,
            amount,
            denom,
            destination_domain_id,
            mint_recipient,
        }
    }
}
