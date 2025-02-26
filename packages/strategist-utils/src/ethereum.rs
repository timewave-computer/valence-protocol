use std::str::FromStr;

use crate::common::{
    base_client::BaseClient, error::StrategistError, transaction::TransactionResponse,
};
use alloy::primitives::Address;
use alloy::providers::{
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
    Identity, Provider, ProviderBuilder, RootProvider,
};
use cosmrs::Coin;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tonic::async_trait;

pub struct EthereumClient {
    rpc_url: String,
    mnemonic: String,
    chain_id: u64,
}

impl EthereumClient {
    pub fn new(rpc_url: &str, mnemonic: &str, chain_id: u64) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            mnemonic: mnemonic.to_string(),
            chain_id,
        }
    }

    async fn get_client(
        &self,
    ) -> Result<
        FillProvider<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            RootProvider,
        >,
        StrategistError,
    > {
        let provider = ProviderBuilder::new().on_http(self.rpc_url.parse().unwrap());
        Ok(provider)
    }
}

#[async_trait]
impl BaseClient for EthereumClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError> {
        let client = self.get_client().await?;
        let block = client.get_block_number().await.unwrap();

        Ok(block)
    }

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError> {
        let client = self.get_client().await?;

        let addr = Address::from_str(address).unwrap();
        let balance = client.get_balance(addr).await.unwrap();
        Ok(balance.to_string().parse().unwrap())
    }

    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: Value,
    ) -> Result<T, StrategistError> {
        unimplemented!()
    }

    async fn transfer(
        &self,
        to: &str,
        amount: u128,
        denom: &str,
        options: Option<String>,
    ) -> Result<TransactionResponse, StrategistError> {
        unimplemented!()
    }

    async fn execute_wasm<T: Serialize + Send + 'static>(
        &self,
        contract: &str,
        msg: T,
        funds: Vec<Coin>,
    ) -> Result<TransactionResponse, StrategistError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These would be replaced with actual test values
    const TEST_RPC_URL: &str = "http://127.0.0.1:8545";
    const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
    const TEST_CHAIN_ID: u64 = 31337;
    const TEST_ADDR_1: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    const TEST_ADDR_2: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

    #[tokio::test]
    async fn test_eth_latest_block_height() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC, TEST_CHAIN_ID);

        let block_number = client.latest_block_height().await.unwrap();

        println!("block number: {:?}", block_number);
    }

    #[tokio::test]
    async fn test_eth_query_balance() {
        let client = EthereumClient::new(TEST_RPC_URL, TEST_MNEMONIC, TEST_CHAIN_ID);

        let balance = client.query_balance(TEST_ADDR_1, "").await.unwrap();

        println!("balance: {:?}", balance);
    }
}
