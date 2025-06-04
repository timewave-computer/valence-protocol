use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128};
use cw_ownable::cw_ownable_query;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    /// Message to lend tokens.
    Lend {},
    /// Message to withdraw tokens. If amount is not specified, full amount will be withdrawn.
    Withdraw { amount: Option<Uint128> },
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
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    // Address of the pool contract
    pub pool_addr: String,
    // Denom of the asset we are going to lend
    pub denom: String,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        pool_address: String,
        denom: String,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            pool_addr: pool_address,
            denom,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr, Addr, Addr), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;
        let pool_addr = api.addr_validate(&self.pool_addr)?;

        Ok((input_addr, output_addr, pool_addr))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, pool_address) = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            pool_addr: pool_address,
            denom: self.denom.clone(),
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

        // Next update pool_addr (if needed)
        if let Some(pool_addr) = self.pool_addr {
            config.pool_addr = deps.api.addr_validate(&pool_addr)?;
        }

        // Next update denom (if needed)
        if let Some(denom) = self.denom {
            config.denom = denom;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub pool_addr: Addr,
    pub denom: String,
}

impl Config {
    pub fn new(
        input_address: Addr,
        output_address: Addr,
        pool_address: Addr,
        denom: String,
    ) -> Self {
        Config {
            input_addr: input_address,
            output_addr: output_address,
            pool_addr: pool_address,
            denom,
        }
    }
}
