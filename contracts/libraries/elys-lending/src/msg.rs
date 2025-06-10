use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps, DepsMut, Uint128, Uint64};
use cw_ownable::cw_ownable_query;
use valence_lending_utils::elys::ElysQuery;
use valence_library_utils::LibraryAccountType;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub enum FunctionMsgs {
    /// Message to lend tokens.
    Lend {},
    /// Message to withdraw tokens. If amount is not specified, full amount will be withdrawn.
    Withdraw { amount: Option<Uint128> },
    /// Message to claim reward tokens.
    ClaimRewards {},
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
    /// Address of the input account
    pub input_addr: LibraryAccountType,
    /// Address of the output account
    pub output_addr: LibraryAccountType,
    /// ID of the pool we are going to lend to
    pub pool_id: Uint64,
}

impl LibraryConfig {
    pub fn new(
        input_addr: impl Into<LibraryAccountType>,
        output_addr: impl Into<LibraryAccountType>,
        pool: Uint64,
    ) -> Self {
        LibraryConfig {
            input_addr: input_addr.into(),
            output_addr: output_addr.into(),
            pool_id: pool,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, Addr, Uint64), LibraryError> {
        let input_addr = self.input_addr.to_addr(api)?;
        let output_addr = self.output_addr.to_addr(api)?;

        Ok((input_addr, output_addr, self.pool_id))
    }
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (input_addr, output_addr, pool) = self.do_validate(deps.api)?;

        Ok(Config {
            input_addr,
            output_addr,
            pool_id: pool,
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut<ElysQuery>) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        if let Some(input_addr) = self.input_addr {
            config.input_addr = input_addr.to_addr(deps.api)?;
        }

        if let Some(output_addr) = self.output_addr {
            config.output_addr = output_addr.to_addr(deps.api)?;
        }

        if let Some(pool_id) = self.pool_id {
            valence_lending_utils::elys::query_pool(&deps, pool_id.u64())?;
            config.pool_id = pool_id;
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}

#[cw_serde]
pub struct Config {
    pub input_addr: Addr,
    pub output_addr: Addr,
    pub pool_id: Uint64,
}

impl Config {
    pub fn new(input_addr: Addr, output_addr: Addr, pool: Uint64) -> Self {
        Config {
            input_addr,
            output_addr,
            pool_id: pool,
        }
    }
}
