use std::num::ParseIntError;

use aho_corasick::AhoCorasick;

use cosmwasm_schema::schemars;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{to_json_binary, Binary, Empty, StdError};
use serde::{Deserialize, Serialize};
use serde_json::to_vec;
use strum::VariantNames;
use thiserror::Error;

use valence_library_utils::{
    msg::{InstantiateMsg, LibraryConfigValidation},
    Id,
};
use valence_macros::manager_impl_library_configs;

use crate::domain::Domain;

pub type LibraryResult<T> = Result<T, LibraryError>;

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("AhoCorasick Error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),

    #[error("serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("cosmwasm_std Error: {0}")]
    CosmwasmStdError(#[from] StdError),

    #[error("ParseIntError Error: {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("ValenceLibraryError Error: {0}")]
    ValenceLibraryError(#[from] valence_library_utils::error::LibraryError),

    #[error("Tried to compare 2 different configs: {0} and {1}")]
    ConfigsMismatch(String, String),

    #[error("No library config")]
    NoLibraryConfig,

    #[error("No library config update")]
    NoLibraryConfigUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub struct LibraryInfo {
    pub name: String,
    pub domain: Domain,
    pub config: LibraryConfig,
    pub addr: Option<String>,
}

impl LibraryInfo {
    pub fn new(name: String, domain: &Domain, config: LibraryConfig) -> Self {
        Self {
            name,
            domain: domain.clone(),
            config,
            addr: None,
        }
    }
}

/// This is a list of all our libraries we support and their configs.
#[manager_impl_library_configs]
#[derive(
    Debug,
    Clone,
    strum::Display,
    Serialize,
    Deserialize,
    VariantNames,
    PartialEq,
    Default,
    JsonSchema,
)]
#[strum(serialize_all = "snake_case")]
pub enum LibraryConfig {
    #[default]
    None,
    ValenceForwarderLibrary(valence_forwarder_library::msg::LibraryConfig),
    ValenceSplitterLibrary(valence_splitter_library::msg::LibraryConfig),
    ValenceReverseSplitterLibrary(valence_reverse_splitter_library::msg::LibraryConfig),
    ValenceAstroportLper(valence_astroport_lper::msg::LibraryConfig),
    ValenceAstroportWithdrawer(valence_astroport_withdrawer::msg::LibraryConfig),
    ValenceOsmosisGammLper(valence_osmosis_gamm_lper::msg::LibraryConfig),
    ValenceOsmosisGammWithdrawer(valence_osmosis_gamm_withdrawer::msg::LibraryConfig),
    ValenceGenericIbcTransferLibrary(valence_generic_ibc_transfer_library::msg::LibraryConfig),
    ValenceNeutronIbcTransferLibrary(valence_neutron_ibc_transfer_library::msg::LibraryConfig),
    ValenceOsmosisClLper(valence_osmosis_cl_lper::msg::LibraryConfig),
    ValenceOsmosisClWithdrawer(valence_osmosis_cl_withdrawer::msg::LibraryConfig),
    ValenceDropLiquidStaker(valence_drop_liquid_staker::msg::LibraryConfig),
    ValenceDropLiquidUnstaker(valence_drop_liquid_unstaker::msg::LibraryConfig),
    ValenceMarsLending(valence_mars_lending::msg::LibraryConfig),
    ValenceNolusLending(valence_nolus_lending::msg::LibraryConfig),
}

impl LibraryConfig {
    /// Helper to find account ids in the json string
    fn find_account_ids(ac: AhoCorasick, json: String) -> LibraryResult<Vec<Id>> {
        // We find all the places `"|account_id|": is used
        let res = ac.find_iter(&json);
        let mut account_ids = vec![];

        // LOist of all matches
        for mat in res {
            // we take a substring from our match to the next 5 characters
            // we loop over those characters and see if they are numbers
            // once we found a char that is not a number we stop
            // we get Vec<char> and convert it to a string and parse to Id (u64)
            let number = json[mat.end()..]
                .chars()
                .map_while(|char| if char.is_numeric() { Some(char) } else { None })
                .collect::<String>()
                .parse::<Id>()?;
            account_ids.push(number);
        }

        Ok(account_ids)
    }
}
