use serde::{Deserialize, Serialize};

use crate::{
    ethereum::ethereum_config::EthereumStrategyConfig, gaia::gaia_config::GaiaStrategyConfig,
    neutron::neutron_config::NeutronStrategyConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub ethereum: EthereumStrategyConfig,
    pub neutron: NeutronStrategyConfig,
    pub gaia: GaiaStrategyConfig,
}
