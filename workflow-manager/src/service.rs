use aho_corasick::AhoCorasick;

use serde_json::to_vec;
use service_base::msg::InstantiateMsg;
use service_utils::ServiceConfigInterface;
use thiserror::Error;
use valence_reverse_splitter::msg::ServiceConfig as ReverseSplitterServiceConfig;
use valence_splitter::msg::ServiceConfig as SplitterServiceConfig;

use crate::domain::Domain;

pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("AhoCorasick Error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),

    #[error("serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub domain: Domain,
    pub config: ServiceConfig,
}

/// This is a list of all our services we support and their configs.
#[derive(Debug, Clone, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum ServiceConfig {
    // General {
    //     config: GeneralServiceConfig,
    // },
    /// 1 to many
    Splitter(SplitterServiceConfig),

    /// Many to 1
    ReverseSplitter(ReverseSplitterServiceConfig),
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

impl ServiceConfig {
    pub fn is_diff(&self, other: &ServiceConfig) -> bool {
        match (self, other) {
            (ServiceConfig::Splitter(config), ServiceConfig::Splitter(other_config)) => {
                config.is_diff(other_config)
            }
            (
                ServiceConfig::ReverseSplitter(config),
                ServiceConfig::ReverseSplitter(other_config),
            ) => config.is_diff(other_config),
            _ => false,
        }
    }

    pub fn replace_config(
        &mut self,
        patterns: Vec<String>,
        replace_with: Vec<String>,
    ) -> ServiceResult<&mut Self> {
        let ac = AhoCorasick::new(patterns)?;

        match self {
            ServiceConfig::Splitter(ref mut config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            }
            ServiceConfig::ReverseSplitter(config) => {
                let json = serde_json::to_string(&config)?;
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res)?;
            } // ServiceConfig::GlobalSplitter { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
              // ServiceConfig::Lper { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
              // ServiceConfig::Lwer { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
              // ServiceConfig::Forwarder { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
        }

        Ok(self)
    }

    pub fn get_instantiate_msg(&self, owner: String, processor: String) -> ServiceResult<Vec<u8>> {
        match self {
            ServiceConfig::Splitter(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
            ServiceConfig::ReverseSplitter(config) => to_vec(&InstantiateMsg {
                owner,
                processor,
                config: config.clone(),
            }),
        }
        .map_err(ServiceError::SerdeJsonError)
    }
}
