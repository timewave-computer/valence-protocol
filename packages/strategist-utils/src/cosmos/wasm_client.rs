use async_trait::async_trait;
use cosmrs::Coin;
use serde::{de::DeserializeOwned, Serialize};

use crate::common::{error::StrategistError, transaction::TransactionResponse};

#[async_trait]
pub trait WasmClient {
    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: (impl Serialize + Send),
    ) -> Result<T, StrategistError>;

    async fn execute_wasm<T: Serialize + Send + 'static>(
        &self,
        contract: &str,
        msg: T,
        funds: Vec<Coin>,
    ) -> Result<TransactionResponse, StrategistError>;
}
