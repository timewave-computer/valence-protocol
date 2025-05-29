use std::{error::Error, path::Path};

use valence_domain_clients::clients::{
    ethereum::EthereumClient, gaia::CosmosHubClient, neutron::NeutronClient,
};
use valence_e2e::utils::worker::ValenceWorkerTomlSerde;

use crate::{
    ethereum::ethereum_config::EthereumStrategyConfig, gaia::gaia_config::GaiaStrategyConfig,
    neutron::neutron_config::NeutronStrategyConfig,
};

use serde::{Deserialize, Serialize};

/// top-level config that wraps around each domain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub ethereum: EthereumStrategyConfig,
    pub neutron: NeutronStrategyConfig,
    pub gaia: GaiaStrategyConfig,
}

// main strategy struct that wraps around the StrategyConfig
// and stores the initialized clients
pub struct Strategy {
    /// top level strategy configuration
    pub cfg: StrategyConfig,

    /// active ethereum client
    pub(crate) eth_client: EthereumClient,
    /// active cosmos hub client
    pub(crate) gaia_client: CosmosHubClient,
    /// active neutron client
    pub(crate) neutron_client: NeutronClient,
}

impl Strategy {
    /// strategy initializer that takes in a `StrategyConfig`, and uses it
    /// to initialize the respective domain clients. prerequisite to starting
    /// the strategist.
    pub async fn new(cfg: StrategyConfig) -> Result<Self, Box<dyn Error>> {
        let gaia_client = CosmosHubClient::new(
            &cfg.gaia.grpc_url,
            &cfg.gaia.grpc_port,
            &cfg.gaia.mnemonic,
            &cfg.gaia.chain_id,
            &cfg.gaia.denom,
        )
        .await?;

        let neutron_client = NeutronClient::new(
            &cfg.neutron.grpc_url,
            &cfg.neutron.grpc_port,
            &cfg.neutron.mnemonic,
            &cfg.neutron.chain_id,
        )
        .await?;

        let eth_client = EthereumClient::new(&cfg.ethereum.rpc_url, &cfg.ethereum.mnemonic, None)?;

        Ok(Self {
            cfg,
            eth_client,
            gaia_client,
            neutron_client,
        })
    }

    /// constructor helper that takes in three paths:
    /// - neutron config path
    /// - ethereum config path
    /// - cosmos hub config path
    ///
    /// reads the configs from those paths, sets up each domain config,
    /// wraps them in a `StrategyConfig`, and uses that to call the initializer above.
    pub async fn from_files<P: AsRef<Path>>(
        neutron_path: P,
        gaia_path: P,
        eth_path: P,
    ) -> Result<Self, Box<dyn Error>> {
        let neutron_cfg = NeutronStrategyConfig::from_file(neutron_path)?;
        let eth_cfg = EthereumStrategyConfig::from_file(eth_path)?;
        let gaia_cfg = GaiaStrategyConfig::from_file(gaia_path)?;

        let strategy_cfg = StrategyConfig {
            ethereum: eth_cfg,
            neutron: neutron_cfg,
            gaia: gaia_cfg,
        };

        Self::new(strategy_cfg).await
    }
}
