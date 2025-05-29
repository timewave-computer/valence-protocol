use std::{error::Error, str::FromStr};

use alloy::primitives::Address;
use async_trait::async_trait;
use log::info;
use valence_domain_clients::evm::request_provider_client::RequestProviderClient;
use valence_e2e::utils::{
    solidity_contracts::{
        sol_authorizations::Authorizations, sol_lite_processor::LiteProcessor, BaseAccount,
        IBCEurekaTransfer, OneWayVault, ERC20,
    },
    worker::ValenceWorker,
};

use crate::strategy_config::Strategy;

// implement the ValenceWorker trait for the Strategy struct.
// This trait defines the main loop of the strategy and inherits
// the default implementation for spawning the worker.
#[async_trait]
impl ValenceWorker for Strategy {
    fn get_name(&self) -> String {
        "Valence X-Vault: ETH-NEUTRON BTC".to_string()
    }

    async fn cycle(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let worker_name = self.get_name();
        info!("{worker_name}: Starting cycle...");

        let eth_rp = self.eth_client.get_request_provider().await?;

        // ======================= ETH Side setup =============================
        // here we build up the Ethereum domain state for the strategy cycle
        let eth_authorizations_contract = Authorizations::new(
            Address::from_str(&self.cfg.ethereum.authorizations)?,
            &eth_rp,
        );
        let eth_processor_contract =
            LiteProcessor::new(Address::from_str(&self.cfg.ethereum.processor)?, &eth_rp);
        let eth_deposit_acc_contract = BaseAccount::new(
            Address::from_str(&self.cfg.ethereum.accounts.deposit)?,
            &eth_rp,
        );
        let eth_wbtc_contract =
            ERC20::new(Address::from_str(&self.cfg.ethereum.denoms.wbtc)?, &eth_rp);
        let eth_one_way_vault_contract = OneWayVault::new(
            Address::from_str(&self.cfg.ethereum.libraries.one_way_vault)?,
            &eth_rp,
        );
        let eth_eureka_transfer_contract = IBCEurekaTransfer::new(
            Address::from_str(&self.cfg.ethereum.libraries.eureka_forwarder)?,
            &eth_rp,
        );

        Ok(())
    }
}
