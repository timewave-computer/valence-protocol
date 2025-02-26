use crate::common::{
    base_client::BaseClient, error::StrategistError, transaction::TransactionResponse,
};
use async_trait::async_trait;

use cosmos_sdk_proto::cosmos::bank::v1beta1::QueryBalanceResponse;
use cosmrs::proto::{
    cosmos::{
        bank::v1beta1::QueryBalanceRequest,
        base::tendermint::v1beta1::{
            service_client::ServiceClient as TendermintServiceClient, GetLatestBlockRequest,
        },
        tx::v1beta1::{service_client::ServiceClient as TxServiceClient, BroadcastTxRequest},
    },
    cosmwasm::wasm::v1::QuerySmartContractStateRequest,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tonic::{transport::Channel, Request};

pub struct NeutronClient {
    grpc_url: String,
}

impl NeutronClient {
    pub fn new(rpc_url: &str, rpc_port: &str) -> Self {
        Self {
            grpc_url: format!("{rpc_url}:{rpc_port}"),
        }
    }

    pub async fn get_grpc_channel(&self) -> Result<Channel, StrategistError> {
        Channel::from_shared(self.grpc_url.clone())
            .map_err(|e| StrategistError::ClientError("failed to build channel".to_string()))?
            .connect()
            .await
            .map_err(|e| StrategistError::ClientError("failed to connect to channel".to_string()))
    }
}

#[async_trait]
impl BaseClient for NeutronClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut tendermint_client = TendermintServiceClient::new(channel);

        let response = tendermint_client
            .get_latest_block(GetLatestBlockRequest {})
            .await
            .map_err(|e| StrategistError::QueryError(e.to_string()))?
            .into_inner();

        let sdk_block = response
            .sdk_block
            .ok_or_else(|| StrategistError::QueryError("no block in response".to_string()))?;

        let block_header = sdk_block
            .header
            .ok_or_else(|| StrategistError::QueryError("no header in sdk_block".to_string()))?;

        let height = u64::try_from(block_header.height)
            .map_err(|_| StrategistError::ParseError("failed to get height".to_string()))?;

        Ok(height)
    }

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut bank_client =
            cosmrs::proto::cosmos::bank::v1beta1::query_client::QueryClient::new(channel);

        let request = QueryBalanceRequest {
            address: address.to_string(),
            denom: denom.to_string(),
        };

        let response: QueryBalanceResponse = bank_client
            .balance(Request::new(request))
            .await
            .map_err(|e| StrategistError::QueryError(e.to_string()))?
            .into_inner();

        let coin = response
            .balance
            .ok_or_else(|| StrategistError::QueryError("No balance returned".to_string()))?;

        let amount = coin
            .amount
            .parse::<u128>()
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        Ok(amount)
    }

    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: Value,
    ) -> Result<T, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut wasm_client =
            cosmrs::proto::cosmwasm::wasm::v1::query_client::QueryClient::new(channel);

        let bin_query = serde_json::to_vec(&query_data)
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        let request = QuerySmartContractStateRequest {
            address: contract_address.to_string(),
            query_data: bin_query,
        };

        let response = wasm_client
            .smart_contract_state(Request::new(request))
            .await
            .map_err(|e| StrategistError::QueryError(e.to_string()))?
            .into_inner();

        let parsed: T = serde_json::from_slice(&response.data)
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        Ok(parsed)
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

    const GRPC_URL: &str = "-"; // update during dev to a real one
    const GRPC_PORT: &str = "19190";
    const NEUTRON_DAO_ADDR: &str =
        "neutron1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrstdxvff";
    #[tokio::test]
    async fn test_latest_block_height() {
        let client = NeutronClient::new(GRPC_URL, GRPC_PORT);

        let block_height = client
            .latest_block_height()
            .await
            .expect("Failed to get latest block height");

        println!("latest block height: {}", block_height);
    }

    #[tokio::test]
    async fn test_query_balance() {
        let client = NeutronClient::new(GRPC_URL, GRPC_PORT);
        let balance = client
            .query_balance(NEUTRON_DAO_ADDR, "untrn")
            .await
            .unwrap();

        println!("balance: {}", balance);
    }

    #[tokio::test]
    async fn test_query_contract_state() {
        let client = NeutronClient::new(GRPC_URL, GRPC_PORT);

        let query_data = serde_json::json!({
          "config": {}
        });

        let state: Value = client
            .query_contract_state(NEUTRON_DAO_ADDR, query_data)
            .await
            .unwrap();
    }
}
