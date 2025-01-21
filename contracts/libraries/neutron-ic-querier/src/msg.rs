use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Binary, Deps, Uint64};
use cw_ownable::cw_ownable_query;

use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

use crate::contract::ExecuteDeps;

#[cw_serde]
pub enum FunctionMsgs {
    RegisterKvQuery { target_query: String },
    DeregisterKvQuery { query_id: u64 },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<(u64, String)>)]
    RegisteredQueries {},
}

#[cw_serde]
pub struct QuerierConfig {
    pub broker_addr: String,
    pub connection_id: String,
    // TODO: add known query configurations
}

#[cw_serde]
#[derive(ValenceLibraryInterface)]
pub struct LibraryConfig {
    pub storage_account: LibraryAccountType,
    pub querier_config: QuerierConfig,
    pub query_definitions: BTreeMap<String, QueryDefinition>,
}

#[cw_serde]
pub struct QueryDefinition {
    pub registry_version: Option<String>,
    pub type_id: String,
    pub update_period: Uint64,
    pub params: BTreeMap<String, Binary>,
}

impl LibraryConfig {
    pub fn new(
        storage_acc: impl Into<LibraryAccountType>,
        querier_config: QuerierConfig,
        query_definitions: BTreeMap<String, QueryDefinition>,
    ) -> Self {
        LibraryConfig {
            storage_account: storage_acc.into(),
            querier_config,
            query_definitions,
        }
    }

    fn do_validate(
        &self,
        api: &dyn cosmwasm_std::Api,
    ) -> Result<(Addr, QuerierConfig, BTreeMap<String, QueryDefinition>), LibraryError> {
        let storage_addr = self.storage_account.to_addr(api)?;
        let querier_config = self.querier_config.clone();
        let query_definitions = self.query_definitions.clone();

        Ok((storage_addr, querier_config, query_definitions))
    }
}

/// Validated library configuration
#[cw_serde]
pub struct Config {
    pub storage_acc_addr: Addr,
    pub querier_config: QuerierConfig,
    pub query_definitions: BTreeMap<String, QueryDefinition>,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let (storage_acc_addr, querier_config, query_definitions) = self.do_validate(deps.api)?;

        Ok(Config {
            storage_acc_addr,
            querier_config,
            query_definitions,
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: ExecuteDeps) -> Result<(), LibraryError> {
        let config: Config = valence_library_base::load_config(deps.storage)?;
        // TODO
        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}
