use async_trait::async_trait;
use cosmrs::Coin;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub mod errors;

use crate::common::{error::StrategistError, transaction::TransactionResponse};

#[async_trait]
pub trait BaseClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError>;

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError>;

    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: Value,
    ) -> Result<T, StrategistError>;

    async fn transfer(
        &self,
        to: &str,
        amount: u128,
        denom: &str,
        options: Option<String>,
    ) -> Result<TransactionResponse, StrategistError>;

    async fn execute_wasm<T: Serialize + Send + 'static>(
        &self,
        contract: &str,
        msg: T,
        funds: Vec<Coin>,
    ) -> Result<TransactionResponse, StrategistError>;
}
