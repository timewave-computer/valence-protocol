use std::{error::Error, path::Path};

use alloy::signers::local::{coins_bip39::English, MnemonicBuilder};
use async_trait::async_trait;
use localic_utils::NEUTRON_CHAIN_ID;

use valence_chain_client_utils::{ethereum::EthereumClient, neutron::NeutronClient};
use valence_e2e::utils::worker::{ValenceWorker, ValenceWorkerTomlSerde};

use super::strategy_config::StrategyConfig;

// main strategy struct that wraps around the StrategyConfig
// and stores the initialized clients
pub struct Strategy {
    pub cfg: StrategyConfig,

    pub(crate) eth_client: EthereumClient,
    pub(crate) neutron_client: NeutronClient,
}

impl Strategy {
    // async constructor which initializes the clients baesd on the StrategyConfig
    pub async fn new(cfg: StrategyConfig) -> Result<Self, Box<dyn Error>> {
        let neutron_client = NeutronClient::new(
            &cfg.neutron.grpc_url,
            &cfg.neutron.grpc_port,
            &cfg.neutron.mnemonic,
            NEUTRON_CHAIN_ID,
        )
        .await?;

        let eth_client = EthereumClient {
            rpc_url: cfg.ethereum.rpc_url.to_string(),
            signer: MnemonicBuilder::<English>::default()
                .phrase(cfg.ethereum.mnemonic.clone())
                .index(7)? // derive the mnemonic at a different index to avoid nonce issues
                .build()?,
        };

        Ok(Strategy {
            cfg,
            // store the initialized clients
            eth_client,
            neutron_client,
        })
    }

    // initialization helper that parses StrategyConfig from a file and calls the
    // default constructor (`Strategy::new`)
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let strategy_cfg = StrategyConfig::from_file(path)?;
        Self::new(strategy_cfg).await
    }
}

// implement the ValenceWorker trait for the Strategy struct.
// This trait defines the main loop of the strategy and inherits
// the default implementation for spawning the worker.
#[async_trait]
impl ValenceWorker for Strategy {
    fn get_name(&self) -> String {
        "Valence X-Vault: ETH-NEUTRON".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        unimplemented!()
    }
}
