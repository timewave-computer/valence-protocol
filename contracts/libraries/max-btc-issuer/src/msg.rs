use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    Issue {},
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
    pub input_addr: LibraryAccountType,
    pub output_addr: LibraryAccountType,
    // Address of the maxBTC issuer contract
    pub maxbtc_issuer_addr: String,
    // Denom of the BTC derivative we are going to deposit
    pub btc_denom: String,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        maxbtc_issuer_addr: String,
        btc_denom: String,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            maxbtc_issuer_addr,
            btc_denom,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let maxbtc_issuer_addr = api.addr_validate(&self.maxbtc_issuer_addr)?;

        Ok((input_addr, output_addr, maxbtc_issuer_addr))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, maxbtc_issuer_addr) = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            maxbtc_issuer_addr,
            btc_denom: self.btc_denom.clone(),
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

        // Next update output_addr (if needed)
        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_addr(deps.api)?;
        }

        // Next update maxbtc_issuer_addr (if needed)
        if let Some(maxbtc_issuer_addr) = self.maxbtc_issuer_addr {
            config.maxbtc_issuer_addr = deps.api.addr_validate(&maxbtc_issuer_addr)?;
        }

        // Next update btc_denom (if needed)
        if let Some(btc_denom) = self.btc_denom {
            config.btc_denom = btc_denom;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub maxbtc_issuer_addr: Addr,
    pub btc_denom: String,
}

impl Config {
    pub fn new(
        input_addr: Addr,
        output_addr: Addr,
        maxbtc_issuer_addr: Addr,
        btc_denom: String,
    ) -> Self {
        Config {
            input_addr,
            output_addr,
            maxbtc_issuer_addr,
            btc_denom,
        }
    }
}
