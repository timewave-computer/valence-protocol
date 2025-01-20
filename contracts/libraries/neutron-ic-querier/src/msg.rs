use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, Binary, Deps, Uint64};
use cw_ownable::cw_ownable_query;

use valence_library_utils::{
    error::LibraryError, msg::LibraryConfigValidation, LibraryAccountType,
};
use valence_macros::{valence_library_query, ValenceLibraryInterface};
use valence_middleware_utils::type_registry::types::ValenceType;

use crate::contract::ExecuteDeps;

#[cw_serde]
pub enum FunctionMsgs {
    RegisterKvQuery {
        registry_version: Option<String>,
        type_id: String,
        update_period: Uint64,
        params: BTreeMap<String, Binary>,
    },
}

#[valence_library_query]
#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<(u64, String)>)]
    RegisteredQueries {},

    #[returns(Vec<(u64, ValenceType)>)]
    QueryResults {},
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
}

impl LibraryConfig {
    pub fn new(storage_acc: impl Into<LibraryAccountType>, querier_config: QuerierConfig) -> Self {
        LibraryConfig {
            storage_account: storage_acc.into(),
            querier_config,
        }
    }

    fn do_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(Addr), LibraryError> {
        let storage_addr = self.storage_account.to_addr(api)?;
        Ok((storage_addr))
    }
}

#[cw_serde]
/// Validated library configuration
pub struct Config {
    pub storage_acc_addr: Addr,
    pub querier_config: QuerierConfig,
}

impl LibraryConfigValidation<Config> for LibraryConfig {
    #[cfg(not(target_arch = "wasm32"))]
    fn pre_validate(&self, api: &dyn cosmwasm_std::Api) -> Result<(), LibraryError> {
        self.do_validate(api)?;
        Ok(())
    }

    fn validate(&self, deps: Deps) -> Result<Config, LibraryError> {
        let storage_acc_addr = self.do_validate(deps.api)?;

        Ok(Config {
            storage_acc_addr,
            querier_config: self.querier_config.clone(),
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
