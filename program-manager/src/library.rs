use std::{num::ParseIntError, str::FromStr};

use aho_corasick::AhoCorasick;

use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::{Empty, StdError};
use serde::{Deserialize, Serialize};
use serde_json::to_vec;
use strum::VariantNames;
use thiserror::Error;

use alloy::{
    primitives::Address,
    sol_types::{SolCall, SolConstructor, SolValue},
};
use valence_library_utils::{
    msg::{InstantiateMsg, LibraryConfigValidation},
    Id,
};
use valence_macros::manager_impl_library_configs;
use valence_solidity_bindings::Forwarder as EvmForwarder;

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
    
    #[error("Failed to parse string into EVM Address: {0}")]
    FailedToParseAddress(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[schemars(crate = "cosmwasm_schema::schemars")]
pub struct LibraryInfo {
    pub name: String,
    pub domain: Domain,
    #[schemars(skip)]
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
    Debug, Clone, strum::Display, Serialize, Deserialize, VariantNames, PartialEq, Default,
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
    EvmForwarderLibrary(EvmForwarder::ForwarderConfig),
}
