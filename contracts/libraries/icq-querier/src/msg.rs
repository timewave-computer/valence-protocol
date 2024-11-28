use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Deps, DepsMut};
use cw_ownable::cw_ownable_query;
use valence_library_utils::{error::LibraryError, msg::LibraryConfigValidation};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum FunctionMsgs {
    RegisterKvQuery {
        connection_id: String,
        update_period: u64,
        // TODO: enum
        module: String,
        //
    },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<(String, String)>)]
    Logs {},

    #[returns(Vec<(u64, String)>)]
    RegisteredQueries {},

    #[returns(Vec<(u64, String)>)]
    QueryResults {},
}

#[cw_serde]
pub struct QuerierConfig {}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub querier_config: QuerierConfig,
}

impl LibraryConfig {
    pub fn new(querier_config: QuerierConfig) -> Self {
        LibraryConfig { querier_config }
    }

    fn do_validate(&self, _api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        Ok(())
    }
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub querier_config: QuerierConfig,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        self.do_validate(deps.api)?;

        Ok(Config {
            querier_config: self.querier_config.clone(),
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: DepsMut) -> Result<(), LibraryError> {
        let config: Config = valence_library_base::load_config(deps.storage)?;
        // TODO
        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}
