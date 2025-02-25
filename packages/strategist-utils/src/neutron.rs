use async_trait::async_trait;
use cosmos_sdk_proto::cosmos::bank::v1beta1::QueryBalanceResponse;
use cosmrs::{
    proto::cosmos::bank::v1beta1::QueryBalanceRequest,
    rpc::{Client, HttpClient},
    AccountId,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::common::{
    base_client::BaseClient, error::StrategistError, transaction::TransactionResponse,
};

pub struct NeutronClient {
    url: String,
}

impl NeutronClient {
    pub fn new(rpc_url: &str, rpc_port: &str) -> Result<Self, StrategistError> {
        let rpc_address = format!("{rpc_url}:{rpc_port}");

        Ok(Self { url: rpc_address })
    }

    pub fn get_client(&self) -> HttpClient {
        HttpClient::new(self.url.as_str()).unwrap()
    }
}

#[async_trait]
impl BaseClient for NeutronClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError> {
        let client = self.get_client();

        let info = client
            .abci_info()
            .await
            .map_err(|e| StrategistError::QueryError(e.to_string()))?;

        let latest_block_height = info.last_block_height;

        Ok(latest_block_height.into())
    }

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError> {
        let client = self.get_client();

        let account_id = address
            .parse::<AccountId>()
            .map_err(|e| StrategistError::ParseError(format!("Invalid address: {}", e)))?;
        println!("account id: {:?}", account_id);

        let request = QueryBalanceRequest {
            address: address.to_string(),
            denom: denom.to_string(),
        };

        Ok(0)
    }

    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: Value,
    ) -> Result<T, StrategistError> {
        unimplemented!()
    }

    async fn execute_transaction(
        &self,
        to: &str,
        data: Vec<u8>,
        options: Option<String>,
    ) -> Result<TransactionResponse, StrategistError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RPC_URL: &str = "-"; // update during dev to a real one
    const RPC_PORT: &str = "443";

    #[tokio::test]
    async fn test_latest_block_height() {
        let client = NeutronClient::new(RPC_URL, RPC_PORT).expect("Failed to create client");

        let block_height = client
            .latest_block_height()
            .await
            .expect("Failed to get latest block height");

        println!("latest block height: {}", block_height);
    }

    #[tokio::test]
    async fn test_query_balance() {
        let client = NeutronClient::new(RPC_URL, RPC_PORT).expect("Failed to create client");
        let ntrn_dao = "neutron1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrstdxvff";
        let balance = client.query_balance(ntrn_dao, "untrn").await.unwrap();

        println!("balance: {}", balance);
    }
}
