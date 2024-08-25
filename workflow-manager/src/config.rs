use std::collections::HashMap;

use config::{Config as ConfigHelper, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub chains: HashMap<String, ChainInfo>,
    pub contracts: Contracts,
}

impl Default for Config {
    fn default() -> Self {
        let cfg = ConfigHelper::builder()
            .add_source(
                glob::glob("conf/*")
                    .unwrap()
                    .map(|path| File::from(path.unwrap()))
                    .collect::<Vec<_>>(),
            )
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        println!("{:#?}", cfg);
        cfg

        // TODO: Verify the config is not missing any info
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
pub struct Contracts {
    pub code_ids: HashMap<String, HashMap<String, u64>>,
}

impl Config {
    pub fn get_chain_info(&self, chain_name: String) -> ChainInfo {
        self.chains.get(&chain_name).unwrap().clone()
    }

    pub fn get_code_ids(&self, chain_name: &str) -> HashMap<String, u64> {
        self.contracts.code_ids.get(chain_name).unwrap().clone()
    }
}
