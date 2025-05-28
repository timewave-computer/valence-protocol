use serde::{Deserialize, Serialize};

use crate::neutron::neutron_config::NeutronStrategyConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub ethereum: String,
    pub neutron: NeutronStrategyConfig,
}
