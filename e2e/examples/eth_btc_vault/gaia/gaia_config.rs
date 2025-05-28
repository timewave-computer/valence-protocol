use serde::{Deserialize, Serialize};
use valence_e2e::utils::worker::ValenceWorkerTomlSerde;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaiaStrategyConfig {
    pub grpc_url: String,
    pub grpc_port: String,
    pub chain_id: String,
    pub mnemonic: String,
    pub denom: String,
}

// default impl serde trait to enable toml config file parsing
impl ValenceWorkerTomlSerde for GaiaStrategyConfig {}
