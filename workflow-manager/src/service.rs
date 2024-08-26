use aho_corasick::AhoCorasick;

use serde_json::to_vec;
use service_base::msg::InstantiateMsg;
use services_utils::ServiceConfigInterface;
use valence_reverse_splitter::msg::ServiceConfig as ReverseSplitterServiceConfig;
use valence_splitter::msg::ServiceConfig as SplitterServiceConfig;

use crate::domain::Domain;

#[derive(Debug, Clone)]
pub struct ServiceInfo {
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

    pub fn replace_config(&mut self, patterns: Vec<String>, replace_with: Vec<String>) -> Self {
        let ac = AhoCorasick::new(patterns).unwrap();

        match self {
            ServiceConfig::Splitter(ref mut config) => {
                let json = serde_json::to_string(&config).unwrap();
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res).unwrap();
            }
            ServiceConfig::ReverseSplitter(config) => {
                let json = serde_json::to_string(&config).unwrap();
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res).unwrap();
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

        self.clone()
    }

    pub fn get_instantiate_msg(&self, owner: String, processor: String) -> Vec<u8> {
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
        .unwrap()
    }
}
