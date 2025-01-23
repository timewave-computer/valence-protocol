use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{ensure, Addr, Binary, Deps, Uint64};
use cw_ownable::cw_ownable_query;

use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};

use crate::contract::ExecuteDeps;

#[cw_serde]
pub enum FunctionMsgs {
    RegisterKvQuery { target_query: String },
    DeregisterKvQuery { target_query: String },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

#[cw_serde]
pub struct QuerierConfig {
    pub broker_addr: String,
    pub connection_id: String,
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
    pub type_url: String,
    pub update_period: Uint64,
    pub params: BTreeMap<String, Binary>,
    /// query_id assigned by the `interchainqueries` module.
    /// `None` on initialization, set after query is registered,
    /// `None` after query is deregistered.
    pub query_id: Option<u64>,
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
        // get the storage account address
        let storage_addr = self.storage_account.to_addr(api)?;

        // validate the querier config fields
        ensure!(
            !self.querier_config.connection_id.is_empty(),
            LibraryError::ConfigurationError("connection_id cannot be empty".to_string())
        );
        api.addr_validate(&self.querier_config.broker_addr)?;

        // validate query definitions
        for (_, query_definition) in self.query_definitions.iter() {
            ensure!(
                query_definition.update_period > Uint64::zero(),
                LibraryError::ConfigurationError(
                    "query update period must be greater than 0".to_string()
                )
            );
            ensure!(
                !query_definition.type_url.is_empty(),
                LibraryError::ConfigurationError("query type_url cannot be empty".to_string())
            );
            ensure!(
                query_definition.query_id.is_none(),
                LibraryError::ConfigurationError(
                    "query_id should only be set after query is registered".to_string()
                )
            )
        }

        Ok((
            storage_addr,
            self.querier_config.clone(),
            self.query_definitions.clone(),
        ))
    }
}

/// Validated library configuration
#[cw_serde]
pub struct Config {
    // storage account to which the library will write the results
    pub storage_acc_addr: Addr,
    // querier configurations that apply to all defined queries
    pub querier_config: QuerierConfig,
    // map of configured queries that can be registered.
    // key: query identifier (arbitrary string for internal use)
    // value: query definition containing the necessary information
    // to register the query
    pub query_definitions: BTreeMap<String, QueryDefinition>,
    // map of currently registered interchain queries.
    // key: `interchainqueries` module assigned query_id
    // value: query identifier from `query_definitions` above
    pub registered_queries: BTreeMap<u64, String>,
    // index of queries currently being registered.
    // index of the map is the id being used for submsg reply.
    // value at the given index is the query identifier which should
    // have an associated value in the `query_definitions` above.
    // in practice, this map should "always" be empty because it gets
    // cleared upon submsg callback.
    pub pending_query_registrations: BTreeMap<u64, String>,
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
            registered_queries: BTreeMap::new(),
            pending_query_registrations: BTreeMap::new(),
        })
    }
}

impl LibraryConfigUpdate {
    pub fn update_config(self, deps: ExecuteDeps) -> Result<(), LibraryError> {
        let mut config: Config = valence_library_base::load_config(deps.storage)?;

        if let Some(storage_acc) = self.storage_account {
            config.storage_acc_addr = storage_acc.to_addr(deps.api)?;
        }

        if let Some(querier_config) = self.querier_config {
            ensure!(
                !querier_config.connection_id.is_empty(),
                LibraryError::ConfigurationError("connection_id cannot be empty".to_string())
            );
            deps.api.addr_validate(&querier_config.broker_addr)?;
            config.querier_config = querier_config;
        }

        if let Some(query_definitions) = self.query_definitions {
            for (_, query_definition) in query_definitions.iter() {
                ensure!(
                    query_definition.update_period > Uint64::zero(),
                    LibraryError::ConfigurationError(
                        "query update period must be greater than 0".to_string()
                    )
                );
                ensure!(
                    !query_definition.type_url.is_empty(),
                    LibraryError::ConfigurationError("query type_url cannot be empty".to_string())
                );
            }
        }

        valence_library_base::save_config(deps.storage, &config)?;
        Ok(())
    }
}
