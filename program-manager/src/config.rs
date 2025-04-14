use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::bridge::{BridgeInfo, BridgesConfig, BridgesMap};

pub type ConfigResult<T> = Result<T, ConfigError>;

pub static GLOBAL_CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| Mutex::new(Config::default()));

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Chain not found for: {0}")]
    ChainInfoNotFound(String),

    #[error("Code ids not found for: {0}")]
    CodeIdsNotFound(String),

    #[error("Bridge details not found, from : {0} to : {1}")]
    ChainBridgeNotFoundA(String, String),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub chains: HashMap<String, ChainInfo>,
    pub contracts: Contracts,
    pub bridges: BridgesConfig,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    pub name: String,
    pub rpc: String,
    pub grpc: String,
    pub prefix: String,
    pub gas_price: String,
    pub gas_denom: String,
    pub coin_type: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    pub registry_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

    pub fn get_bridges(&self, chain_a: &str, chain_b: &str) -> ConfigResult<&BridgesMap> {
        self.bridges
            .get(chain_a)
            .ok_or(ConfigError::ChainBridgeNotFoundA(chain_a.to_string(), chain_b.to_string()))?
            .get(chain_b)
            .ok_or(ConfigError::ChainBridgeNotFoundA(chain_a.to_string(), chain_b.to_string()))
    }

    pub fn get_bridge_info(&self, chain_a: &str, chain_b: &str, bridge_name: &str) -> ConfigResult<&BridgeInfo>{
        let bridges_map = self.get_bridges(chain_a, chain_b)?;
        bridges_map
            .get(bridge_name)
            .ok_or(ConfigError::ChainBridgeNotFoundA(chain_a.to_string(), chain_b.to_string()))
    }

    pub fn get_registry_addr(&self) -> String {
        self.general.registry_addr.to_string()
    }

    pub fn update_code_id(&mut self, chain_name: String, contract_name: String, code_id: u64) {
        self.contracts
            .code_ids
            .entry(chain_name)
            .or_default()
            .insert(contract_name, code_id);
    }
}
