use std::num::ParseIntError;

use aho_corasick::AhoCorasick;

use cosmwasm_std::{to_json_binary, Binary, Empty, StdError};
use serde::{Deserialize, Serialize};
use serde_json::to_vec;
use strum::VariantNames;
use thiserror::Error;

use valence_library_utils::{
    msg::{InstantiateMsg, LibraryConfigValidation},
    Id,
};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfo {
    pub name: String,
    pub domain: Domain,
    #[serde(skip)]
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
}

#[derive(
    Debug, Clone, strum::Display, Serialize, Deserialize, VariantNames, PartialEq, Default,
)]
#[strum(serialize_all = "snake_case")]
pub enum LibraryConfigUpdate {
    #[default]
    None,
    ValenceForwarderLibrary(valence_forwarder_library::msg::LibraryConfigUpdate),
    ValenceSplitterLibrary(valence_splitter_library::msg::LibraryConfigUpdate),
    ValenceReverseSplitterLibrary(valence_reverse_splitter_library::msg::LibraryConfigUpdate),
    ValenceAstroportLper(valence_astroport_lper::msg::LibraryConfigUpdate),
    ValenceAstroportWithdrawer(valence_astroport_withdrawer::msg::LibraryConfigUpdate),
}

impl LibraryConfigUpdate {
    pub fn get_update_msg(self) -> LibraryResult<Binary> {
        match self {
            LibraryConfigUpdate::None => return Err(LibraryError::NoLibraryConfigUpdate),
            LibraryConfigUpdate::ValenceForwarderLibrary(service_config_update) => {
                to_json_binary(&valence_library_utils::msg::ExecuteMsg::<
                    Empty,
                    valence_forwarder_library::msg::LibraryConfigUpdate,
                >::UpdateConfig {
                    new_config: service_config_update,
                })
            }
            LibraryConfigUpdate::ValenceSplitterLibrary(service_config_update) => {
                to_json_binary(&valence_library_utils::msg::ExecuteMsg::<
                    Empty,
                    valence_splitter_library::msg::LibraryConfigUpdate,
                >::UpdateConfig {
                    new_config: service_config_update,
                })
            }
            LibraryConfigUpdate::ValenceReverseSplitterLibrary(service_config_update) => {
                to_json_binary(&valence_library_utils::msg::ExecuteMsg::<
                    Empty,
                    valence_reverse_splitter_library::msg::LibraryConfigUpdate,
                >::UpdateConfig {
                    new_config: service_config_update,
                })
            }
            LibraryConfigUpdate::ValenceAstroportLper(service_config_update) => {
                to_json_binary(&valence_library_utils::msg::ExecuteMsg::<
                    Empty,
                    valence_astroport_lper::msg::LibraryConfigUpdate,
                >::UpdateConfig {
                    new_config: service_config_update,
                })
            }
            LibraryConfigUpdate::ValenceAstroportWithdrawer(service_config_update) => {
                to_json_binary(&valence_library_utils::msg::ExecuteMsg::<
                    Empty,
                    valence_astroport_withdrawer::msg::LibraryConfigUpdate,
                >::UpdateConfig {
                    new_config: service_config_update,
                })
            }
        }
        .map_err(LibraryError::CosmwasmStdError)
    }
}

// TODO: create macro for the methods that work the same over all of the configs
// We are delegating a lot of the methods to the specific config, so most of the methods can be under the macro
impl LibraryConfig {
    pub fn replace_config(
        &mut self,
        patterns: Vec<String>,
        replace_with: Vec<String>,
    ) -> LibraryResult<()> {
        let ac = AhoCorasick::new(patterns)?;

        match self {
            LibraryConfig::None => return Err(LibraryError::NoLibraryConfig),
            LibraryConfig::ValenceForwarderLibrary(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
            LibraryConfig::ValenceSplitterLibrary(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
            LibraryConfig::ValenceReverseSplitterLibrary(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
            LibraryConfig::ValenceAstroportLper(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
            LibraryConfig::ValenceAstroportWithdrawer(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
        }

        Ok(())
    }

    pub fn get_instantiate_msg(&self, owner: String, processor: String) -> LibraryResult<Vec<u8>> {
        match self {
            LibraryConfig::None => return Err(LibraryError::NoLibraryConfig),
            LibraryConfig::ValenceForwarderLibrary(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
            LibraryConfig::ValenceSplitterLibrary(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
            LibraryConfig::ValenceReverseSplitterLibrary(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
            LibraryConfig::ValenceAstroportLper(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
            LibraryConfig::ValenceAstroportWithdrawer(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
        }
        .map_err(LibraryError::SerdeJsonError)
    }

    pub fn soft_validate_config(&self, api: &dyn cosmwasm_std::Api) -> LibraryResult<()> {
        match self {
            LibraryConfig::None => Err(LibraryError::NoLibraryConfig),
            LibraryConfig::ValenceForwarderLibrary(config) => {
                config.pre_validate(api)?;
                Ok(())
            }
            LibraryConfig::ValenceSplitterLibrary(config) => {
                config.pre_validate(api)?;
                Ok(())
            }
            LibraryConfig::ValenceReverseSplitterLibrary(config) => {
                config.pre_validate(api)?;
                Ok(())
            }
            LibraryConfig::ValenceAstroportLper(config) => {
                config.pre_validate(api)?;
                Ok(())
            }
            LibraryConfig::ValenceAstroportWithdrawer(config) => {
                config.pre_validate(api)?;
                Ok(())
            }
        }
    }

    pub fn get_account_ids(&self) -> LibraryResult<Vec<Id>> {
        let ac: AhoCorasick = AhoCorasick::new(["\"|account_id|\":"]).unwrap();

        match self {
            LibraryConfig::None => Err(LibraryError::NoLibraryConfig),
            LibraryConfig::ValenceForwarderLibrary(config) => {
                Self::find_account_ids(ac, serde_json::to_string(&config)?)
            }
            LibraryConfig::ValenceSplitterLibrary(config) => {
                Self::find_account_ids(ac, serde_json::to_string(&config)?)
            }
            LibraryConfig::ValenceReverseSplitterLibrary(config) => {
                Self::find_account_ids(ac, serde_json::to_string(&config)?)
            }
            LibraryConfig::ValenceAstroportLper(config) => {
                Self::find_account_ids(ac, serde_json::to_string(&config)?)
            }
            LibraryConfig::ValenceAstroportWithdrawer(config) => {
                Self::find_account_ids(ac, serde_json::to_string(&config)?)
            }
        }
    }

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
