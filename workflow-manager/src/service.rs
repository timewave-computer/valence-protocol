use std::num::ParseIntError;

use aho_corasick::AhoCorasick;

use serde::{Deserialize, Serialize};
use serde_json::to_vec;
use strum::VariantNames;
use thiserror::Error;
use valence_service_utils::{
    msg::{InstantiateMsg, ServiceConfigValidation},
    Id, ServiceConfigInterface,
};

use crate::domain::Domain;

pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("AhoCorasick Error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),

    #[error("serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("ParseIntError Error: {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("ValenceServiceError Error: {0}")]
    ValenceServiceError(#[from] valence_service_utils::error::ServiceError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct ServiceInfo {
    pub name: String,
    pub domain: Domain,
    pub config: ServiceConfig,
    pub addr: Option<String>,
}

/// This is a list of all our services we support and their configs.
#[derive(Debug, Clone, strum::Display, Serialize, Deserialize, VariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum ServiceConfig {
    Forwarder(valence_forwarder_service::msg::ServiceConfig),
    // General {
    //     config: GeneralServiceConfig,
    // },
    // 1 to many
    // Splitter(SplitterServiceConfig),

    // /// Many to 1
    // ReverseSplitter(ReverseSplitterServiceConfig),
    // /// Many to Many
    // Mapper {
    //     config: MapperSplitterServiceConfig,
    // },
    // Lper {
    //     config: LperServiceConfig,
    // },
    // Lwer {
    //     config: LwerServiceConfig,
    // },
    // Forwarder {
    //     config: ForwarderServiceConfig,
    // },
    // Orbital {
    //     config: OrbitalServiceConfig,
    // },
}

// TODO: create macro for the methods that work the same over all of the configs
// We are delegating a lot of the methods to the specific config, so most of the methods can be under the macro
impl ServiceConfig {
    pub fn is_diff(&self, other: &ServiceConfig) -> bool {
        match (self, other) {
            (ServiceConfig::Forwarder(config), ServiceConfig::Forwarder(other_config)) => {
                config.is_diff(other_config)
            } // _ => false,
        }
    }

    pub fn replace_config(
        &mut self,
        patterns: Vec<String>,
        replace_with: Vec<String>,
    ) -> ServiceResult<&mut Self> {
        let ac = AhoCorasick::new(patterns)?;

        match self {
            ServiceConfig::Forwarder(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
        }

        Ok(self)
    }

    pub fn get_instantiate_msg(&self, owner: String, processor: String) -> ServiceResult<Vec<u8>> {
        match self {
            ServiceConfig::Forwarder(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
        }
        .map_err(ServiceError::SerdeJsonError)
    }

    // TODO: Finish validate config
    pub fn soft_validate_config(&self, api: &dyn cosmwasm_std::Api) -> ServiceResult<()> {
        match self {
            ServiceConfig::Forwarder(config) => {
                config.pre_validate(api)?;
                // config.validate();
                Ok(())
            }
        }
    }

    pub fn get_account_ids(&self) -> ServiceResult<Vec<Id>> {
        let ac: AhoCorasick = AhoCorasick::new(["\"|account_id|\":"]).unwrap();

        match self {
            ServiceConfig::Forwarder(config) => {
                Self::find_account_ids(ac, serde_json::to_string(&config)?)
            }
        }
    }

    /// Helper to find account ids in the json string
    fn find_account_ids(ac: AhoCorasick, json: String) -> ServiceResult<Vec<Id>> {
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
