use std::collections::HashMap;

use config::{Config as ConfigHelper, File};
use once_cell::sync::Lazy;
use serde::Deserialize;
use thiserror::Error;

use crate::bridge::Bridge;

pub type ConfigResult<T> = Result<T, ConfigError>;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::default);

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Chain not found for: {0}")]
    ChainInfoNotFound(String),

    #[error("Code ids not found for: {0}")]
    CodeIdsNotFound(String),

    #[error("Bridge details not found for main chain: {0}")]
    MainChainBridgeNotFound(String),

    #[error("Bridge details not found for: {0}")]
    ChainBridgeNotFound(String),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub chains: HashMap<String, ChainInfo>,
    pub contracts: Contracts,
    pub bridges: HashMap<String, HashMap<String, Bridge>>,
    pub general: GeneralConfig,
}

impl Default for Config {
    fn default() -> Self {
        ConfigHelper::builder()
            .add_source(
                glob::glob("conf/*")
                    .unwrap()
                    .filter_map(|path| {
                        let p = path.unwrap();
                        if p.is_dir() {
                            None
                        } else {
                            Some(File::from(p))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .add_source(
                glob::glob("conf/**/*")
                    .unwrap()
                    .filter_map(|path| {
                        let p = path.unwrap();
                        if p.is_dir() {
                            None
                        } else {
                            Some(File::from(p))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainInfo {
    pub name: String,
    pub rpc: String,
    pub grpc: String,
    pub prefix: String,
    pub gas_price: String,
    pub gas_denom: String,
    pub coin_type: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    pub registry_addr: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Contracts {
    pub code_ids: HashMap<String, HashMap<String, u64>>,
}

impl Config {
    pub fn get_chain_info(&self, chain_name: &str) -> ConfigResult<&ChainInfo> {
        self.chains
            .get(chain_name)
            .ok_or(ConfigError::ChainInfoNotFound(chain_name.to_string()))
    }

    pub fn get_code_ids(&self, chain_name: &str) -> ConfigResult<&HashMap<String, u64>> {
        self.contracts
            .code_ids
            .get(chain_name)
            .ok_or(ConfigError::CodeIdsNotFound(chain_name.to_string()))
    }

    pub fn get_bridge_info(&self, main_chain: &str, chain_name: &str) -> ConfigResult<&Bridge> {
        self.bridges
            .get(main_chain)
            .ok_or(ConfigError::MainChainBridgeNotFound(main_chain.to_string()))?
            .get(chain_name)
            .ok_or(ConfigError::ChainBridgeNotFound(chain_name.to_string()))
    }

    pub fn get_registry_addr(&self) -> String {
        self.general.registry_addr.to_string()
    }
}
