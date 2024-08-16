use aho_corasick::AhoCorasick;

use services_utils::ServiceConfigInterface;
use valence_reverse_splitter::msg::ServiceConfig as ReverseSplitterServiceConfig;
use valence_splitter::msg::ServiceConfig as SplitterServiceConfig;

use super::domain::Domains;

pub struct ServiceInfo {
    pub domain: Domains,
    pub config: ServiceConfigs,
}

/// This is a list of all our services we support and their configs.
#[derive(Debug, Clone)]
pub enum ServiceConfigs {
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

impl ServiceConfigs {
    pub fn is_diff(&self, other: &ServiceConfigs) -> bool {
        match (self, other) {
            (ServiceConfigs::Splitter(config), ServiceConfigs::Splitter(other_config)) => {
                config.is_diff(other_config)
            }
            (
                ServiceConfigs::ReverseSplitter(config),
                ServiceConfigs::ReverseSplitter(other_config),
            ) => config.is_diff(other_config),
            _ => false,
        }
    }

    pub fn replace_config(&mut self, patterns: Vec<String>, replace_with: Vec<String>) -> Self {
        let ac = AhoCorasick::new(patterns).unwrap();

        match self {
            ServiceConfigs::Splitter(ref mut config) => {
                let json = serde_json::to_string(&config).unwrap();
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res).unwrap();
            }
            ServiceConfigs::ReverseSplitter(config) => {
                let json = serde_json::to_string(&config).unwrap();
                let res = ac.replace_all(&json, &replace_with);

                *config = serde_json::from_str(&res).unwrap();
            } // ServiceConfigs::GlobalSplitter { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
              // ServiceConfigs::Lper { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
              // ServiceConfigs::Lwer { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
              // ServiceConfigs::Forwarder { config, .. } => {
              //     let json = serde_json::to_string(&config).unwrap();
              //     let res = ac.replace_all(&json, &replace_with);

              //     *config = serde_json::from_str(&res).unwrap();
              // }
        }

        self.clone()
    }
}
