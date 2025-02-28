use std::str::FromStr;

use crate::common::error::StrategistError;
use crate::evm::{CustomProvider, EvmBaseClient};
use alloy::contract::{CallBuilder, CallDecoder};
use alloy::network::Network;
use alloy::primitives::Address;

use alloy::providers::{Provider, ProviderBuilder};

use alloy::rpc::types::{TransactionReceipt, TransactionRequest};

use alloy::transports::Transport;
use alloy_signer_local::coins_bip39::English;
use alloy_signer_local::{MnemonicBuilder, PrivateKeySigner};
use tonic::async_trait;

pub struct EthereumClient {
    rpc_url: String,
    wallet: PrivateKeySigner,
}

impl EthereumClient {
    pub fn new(rpc_url: &str, mnemonic: &str) -> Result<Self, StrategistError> {
        let builder = MnemonicBuilder::<English>::default().phrase(mnemonic);

        let wallet = builder
            .index(0)
            .unwrap()
            .build()
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        println!("wallet: {}", wallet.address());

        Ok(Self {
            rpc_url: rpc_url.to_string(),
            wallet,
        })
    }

    #[allow(dead_code)]
    async fn query<T: Transport + Clone, P, D, N>(
        &self,
        builder: CallBuilder<T, P, D, N>,
    ) -> Result<D::CallOutput, StrategistError>
    where
        P: Provider<T, N>,
        N: Network,
        D: CallDecoder,
        N::TransactionRequest: Into<TransactionRequest>,
        CallBuilder<T, P, D, N>: Clone,
    {
        let client = self.get_request_provider().await?;

        let tx_request: TransactionRequest = builder.clone().into_transaction_request().into();

        let raw_response = client.call(&tx_request).await.unwrap();

        let decoded = builder.decode_output(raw_response, true).unwrap();

        Ok(decoded)
    }
}

#[async_trait]
impl EvmBaseClient for EthereumClient {
    async fn get_request_provider(&self) -> Result<CustomProvider, StrategistError> {
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(self.rpc_url.parse().unwrap());
        Ok(provider)
    }

    async fn latest_block_height(&self) -> Result<u64, StrategistError> {
        let client = self.get_request_provider().await?;
        let block = client.get_block_number().await.unwrap();

        Ok(block)
    }

    async fn query_balance(&self, address: &str) -> Result<u128, StrategistError> {
        let client = self.get_request_provider().await?;

        let addr = Address::from_str(address).unwrap();
        let balance = client.get_balance(addr).await.unwrap();
        Ok(balance.to_string().parse().unwrap())
    }

    async fn execute_tx(
        &self,
        tx: TransactionRequest,
    ) -> Result<TransactionReceipt, StrategistError> {
        let client = self.get_request_provider().await?;

        let tx_response = client
            .send_transaction(tx.from(self.wallet.address()))
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();

        println!("execute tx response: {:?}", tx_response);

        Ok(tx_response)
    }
}

#[cfg(test)]
mod tests {
    use alloy::{network::TransactionBuilder, primitives::U256};
    use valence_e2e::utils::solidity_contracts::{
        MockERC20,
        ValenceVault::{self},
    };

    use super::*;

    // These would be replaced with actual test values
    const TEST_RPC_URL: &str = "http://127.0.0.1:8545";
    const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
    // const TEST_CHAIN_ID: u64 = 31337;
    const TEST_CONTRACT_ADDR: &str = "0x610178dA211FEF7D417bC0e6FeD39F05609AD788";

    #[tokio::test]
    async fn test_eth_latest_block_height() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC).unwrap();

        let block_number = client.latest_block_height().await.unwrap();
        assert_ne!(block_number, 0);
    }

    #[tokio::test]
    async fn test_eth_query_balance() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC).unwrap();
        let provider = client.get_request_provider().await.unwrap();
        let accounts = provider.get_accounts().await.unwrap();

        let balance = client
            .query_balance(&accounts[0].to_string())
            .await
            .unwrap();

        assert_ne!(balance, 0);
    }

    #[tokio::test]
    async fn test_eth_transfer() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC).unwrap();
        let provider = client.get_request_provider().await.unwrap();
        let accounts = provider.get_accounts().await.unwrap();

        let pre_balance = client
            .query_balance(accounts[1].to_string().as_str())
            .await
            .unwrap();

        let transfer_request = TransactionRequest::default()
            .with_to(accounts[1])
            .with_value(U256::from(200));

        client.execute_tx(transfer_request).await.unwrap();

        let post_balance = client
            .query_balance(accounts[1].to_string().as_str())
            .await
            .unwrap();

        assert_eq!(pre_balance + 200, post_balance);
    }

    #[tokio::test]
    async fn test_eth_erc20_transfer_and_query() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC).unwrap();
        let provider = client.get_request_provider().await.unwrap();
        let accounts = provider.get_accounts().await.unwrap();

        let token_1_tx =
            MockERC20::deploy_builder(&provider, "Token1".to_string(), "T1".to_string())
                .into_transaction_request();

        let token_addr = client
            .execute_tx(token_1_tx)
            .await
            .unwrap()
            .contract_address
            .unwrap();

        let token_1 = MockERC20::new(token_addr, provider);

        let mint_token1_tx = token_1
            .mint(accounts[0], U256::from(1000))
            .into_transaction_request();

        client.execute_tx(mint_token1_tx).await.unwrap();

        let pre_balance_call = token_1.balanceOf(accounts[1]);
        let pre_balance = client.query(pre_balance_call).await.unwrap()._0;

        let transfer_request_builder = token_1
            .transfer(accounts[1], U256::from(200))
            .into_transaction_request();

        client.execute_tx(transfer_request_builder).await.unwrap();

        let post_balance_call = token_1.balanceOf(accounts[1]);
        let post_balance = client.query(post_balance_call).await.unwrap()._0;

        assert_eq!(pre_balance + U256::from(200), post_balance);
    }

    #[tokio::test]
    async fn test_eth_query_contract_states() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC).unwrap();
        let provider = client.get_request_provider().await.unwrap();
        let accounts = provider.get_accounts().await.unwrap();

        let contract_addr = Address::from_str(TEST_CONTRACT_ADDR).unwrap();

        let valence_vault = ValenceVault::new(contract_addr, provider);

        let req = valence_vault.lastUpdateTimestamp();

        let response = client.query(req).await.unwrap();

        assert_ne!(0, response._0);

        let req = valence_vault.balanceOf(accounts[0]);
        let response = client.query(req).await.unwrap();
        assert_eq!(U256::from(0), response._0);
    }
}
