use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumStrategyConfig {
    pub rpc_url: String,
    pub mnemonic: String,
    pub denoms: EthereumDenoms,
    pub accounts: EthereumAccounts,
    pub libraries: EthereumLibraries,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumDenoms {
    pub wbtc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumAccounts {
    pub deposit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumLibraries {
    pub one_way_vault: String,
    pub eureka_forwarder: String,
    pub lite_processor: String,
}
