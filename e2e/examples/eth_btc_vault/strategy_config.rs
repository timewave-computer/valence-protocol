use serde::{Deserialize, Serialize};

use crate::{
    ethereum::ethereum_config::EthereumStrategyConfig,
    neutron::neutron_config::NeutronStrategyConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub ethereum: EthereumStrategyConfig,
    pub neutron: NeutronStrategyConfig,
}
