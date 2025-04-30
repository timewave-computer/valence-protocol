use std::{collections::HashSet, error::Error, time::Duration};

use alloy::{
    primitives::{Address, Log},
    providers::Provider,
    rpc::types::Filter,
};
use async_trait::async_trait;
use log::warn;
use valence_chain_client_utils::{
    ethereum::EthereumClient, evm::request_provider_client::RequestProviderClient,
    gaia::CosmosHubClient,
};

use crate::utils::worker::ValenceWorker;

const POLLING_PERIOD: Duration = Duration::from_secs(5);

pub struct RelayerState {
    // last processed block on gaia
    gaia_last_block: i64,
    // gaia rpc address
    gaia_rpc_addr: String,
    // processed events cache to avoid double processing
    eth_processed_events: HashSet<Vec<u8>>,
    // ethereum filter to poll for events
    eth_filter: Filter,
    // ethereum destination erc20 address
    eth_destination_erc20: Address,
}

pub struct MockEurekaRelayerEvmGaia {
    pub state: RelayerState,
    pub runtime: RelayerRuntime,
}

pub struct RelayerRuntime {
    pub eth_client: EthereumClient,
    pub gaia_client: CosmosHubClient,
}

#[async_trait]
impl ValenceWorker for MockEurekaRelayerEvmGaia {
    fn get_name(&self) -> String {
        "Mock Eureka Relayer: ETH-GAIA".to_string()
    }

    /// each eureka relayer cycle will poll both gaia and ethereum for events
    /// that indicate an IBC Eureka transfer. Once such event is picked up on the origin
    /// domain, it will mint the equivalent amount on the destination chain.
    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();

        if let Err(e) = self.poll_gaia().await {
            warn!("{worker_name}: Gaia polling error: {:?}", e);
        }

        if let Err(e) = self.poll_ethereum().await {
            warn!("{worker_name}: Ethereum polling error: {:?}", e);
        }

        tokio::time::sleep(POLLING_PERIOD).await;

        Ok(())
    }
}

impl MockEurekaRelayerEvmGaia {
    async fn poll_ethereum(&mut self) -> Result<(), Box<dyn Error>> {
        let provider = self
            .runtime
            .eth_client
            .get_request_provider()
            .await
            .expect("could not get provider");

        // fetch the logs
        let logs = provider.get_logs(&self.state.eth_filter).await?;

        for log in logs.iter() {
            let event_id = log
                .transaction_hash
                .expect("failed to find tx hash in eth logs")
                .to_vec();
            if self.state.eth_processed_events.insert(event_id) {
                // TODO
            }
        }

        Ok(())
    }

    async fn poll_gaia(&mut self) -> Result<(), Box<dyn Error>> {
        // TODO
        Ok(())
    }
}
