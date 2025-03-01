use async_trait::async_trait;

use crate::common::{error::StrategistError, transaction::TransactionResponse};

#[async_trait]
pub trait BaseClient {
    async fn latest_block_height(&self) -> Result<u64, StrategistError>;

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError>;

    async fn transfer(
        &self,
        to: &str,
        amount: u128,
        denom: &str,
        options: Option<String>,
    ) -> Result<TransactionResponse, StrategistError>;
}
