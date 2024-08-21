use std::collections::HashMap;

use config::{Config, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Cfg {
    pub chains: HashMap<String, ChainInfo>,
}

impl Default for Cfg {
    fn default() -> Self {
        let cfg = Config::builder()
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

impl Cfg {
    pub fn get_chain_info(&self, chain_name: String) -> ChainInfo {
        self.chains.get(&chain_name).unwrap().clone()
    }
}
