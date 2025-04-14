use std::{env, fmt};

use alloy::{
    network::EthereumWallet,
    providers::{ProviderBuilder, WalletProvider},
    signers::local::{LocalSignerError, PrivateKeySigner},
    sol,
};
use anyhow::Context;
use async_trait::async_trait;
use thiserror::Error;

use crate::{
    account::InstantiateAccountData,
    config::{ChainInfo, ConfigError, GLOBAL_CONFIG},
    library::LibraryConfig,
    mock_api::MockApi,
};

use super::{Connector, ConnectorResult};

const ETHEREUM_PK: &str = "d6d3b8f63e797bf559590dce9f27aa2e43ec3a471496af1a7800bf9819df3aac";
const CHAIN_NAME: &str = "ethereum";

#[derive(Error, Debug)]
pub enum EthEvmError {
    #[error(transparent)]
    Error(#[from] anyhow::Error),

    #[error(transparent)]
    ConfigError(#[from] ConfigError),

    #[error(transparent)]
    LocalSignerError(#[from] LocalSignerError),
}

// Valence Base Accounts
sol!(
    #[sol(rpc)]
    BaseAccount,
    "../solidity/out/BaseAccount.sol/BaseAccount.json",
);

sol!(
    #[sol(rpc)]
    Forwarder,
    "../solidity/out/Forwarder.sol/Forwarder.json",
);

pub struct EthEvmConnector {
    provider: alloy::providers::fillers::FillProvider<
        alloy::providers::fillers::JoinFill<
            alloy::providers::Identity,
            alloy::providers::fillers::WalletFiller<EthereumWallet>,
        >,
        alloy::providers::RootProvider<
            alloy::transports::http::Http<alloy::transports::http::Client>,
        >,
        alloy::transports::http::Http<alloy::transports::http::Client>,
        alloy::network::Ethereum,
    >,
    chain_name: String,
}

impl fmt::Debug for EthEvmConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EthEvmConnector").finish_non_exhaustive()
    }
}

impl EthEvmConnector {
    pub async fn new() -> Result<Self, EthEvmError> {
        let gc = GLOBAL_CONFIG.lock().await;
        let chain_info: &ChainInfo = gc.get_chain_info(CHAIN_NAME)?;

        let pk_signer: PrivateKeySigner = env::var("ETHEREUM_PK")
            .unwrap_or(ETHEREUM_PK.to_string())
            .parse()?;
        let wallet = EthereumWallet::from(pk_signer);

        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(chain_info.rpc.parse().context("Failed to parse RPC URL")?);

        Ok(EthEvmConnector {
            provider,
            chain_name: CHAIN_NAME.to_string(),
        })
    }
}

#[async_trait]
impl Connector for EthEvmConnector {
    async fn get_address(
        &mut self,
        program_id: u64,
        contract_name: &str,
        extra_salt: &str,
    ) -> ConnectorResult<(String, Vec<u8>)> {
        let b = &BaseAccount::DEPLOYED_BYTECODE;
        let c = BaseAccount::deploy_builder(
            self.provider.clone(),
            self.provider.default_signer_address(),
            vec![],
        )
        .calculate_create_address();
        // alloy::primitives::Address::create2(&self, salt, init_code_hash)
        // Implement the logic here
        unimplemented!()
    }

    async fn get_address_bridge(
        &mut self,
        sender_addr: &str,
        main_chain: &str,
        sender_chain: &str,
        receiving_chain: &str,
    ) -> ConnectorResult<String> {
        // Implement the logic here
        unimplemented!()
    }

    async fn instantiate_account(
        &mut self,
        program_id: u64,
        processor_addr: String,
        data: &InstantiateAccountData,
    ) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn instantiate_library(
        &mut self,
        program_id: u64,
        processor_addr: String,
        library_id: u64,
        library_config: LibraryConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn instantiate_processor(
        &mut self,
        program_id: u64,
        salt: Vec<u8>,
        admin: String,
        authorization: String,
        polytone_config: Option<valence_processor_utils::msg::PolytoneContracts>,
    ) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn instantiate_processor_bridge_account(
        &mut self,
        processor_addr: String,
        retry: u8,
    ) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn verify_account(&mut self, account_addr: String) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn verify_library(&mut self, library_addr: Option<String>) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn verify_processor(&mut self, processor_addr: String) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    async fn verify_bridge_account(&mut self, bridge_addr: String) -> ConnectorResult<()> {
        // Implement the logic here
        unimplemented!()
    }

    fn get_api(&self) -> &MockApi {
        // Implement the logic here
        unimplemented!()
    }
}
