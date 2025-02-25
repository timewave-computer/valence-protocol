use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{error::StrategistError, transaction::TransactionResponse};

#[async_trait]
pub trait BaseClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError>;

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError>;

    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: Value,
    ) -> Result<T, StrategistError>;

    async fn execute_transaction(
        &self,
        to: &str,
        data: Vec<u8>,
        options: Option<String>,
    ) -> Result<TransactionResponse, StrategistError>;
}
