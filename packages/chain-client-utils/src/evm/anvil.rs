use alloy::primitives::Address;
use alloy::providers::ext::AnvilApi;
use alloy::providers::Provider;
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use std::str::FromStr;
use tonic::async_trait;

use crate::common::error::StrategistError;
use crate::evm::base_client::EvmBaseClient;

/// Extension trait to add Anvil impersonation capabilities
#[async_trait]
pub trait AnvilImpersonationClient: EvmBaseClient {
    /// Start impersonating an account - calls anvil_impersonateAccount
    async fn impersonate_account(&self, address: &str) -> Result<(), StrategistError> {
        let client = self.get_request_provider().await?;

        // Convert string address to Address type
        let addr = Address::from_str(address)?;

        // Call the Anvil-specific RPC method
        client.anvil_impersonate_account(addr).await.map_err(|e| {
            StrategistError::ClientError(format!("Failed to impersonate account: {}", e))
        })?;

        Ok(())
    }

    /// Stop impersonating an account - calls anvil_stopImpersonatingAccount
    async fn stop_impersonating_account(&self, address: &str) -> Result<(), StrategistError> {
        let client = self.get_request_provider().await?;

        // Convert string address to Address type
        let addr = Address::from_str(address)?;

        // Call the Anvil-specific RPC method
        client
            .anvil_stop_impersonating_account(addr)
            .await
            .map_err(|e| {
                StrategistError::ClientError(format!("Failed to stop impersonating account: {}", e))
            })?;

        Ok(())
    }

    /// Execute transaction as impersonated account
    async fn execute_tx_as(
        &self,
        from_address: &str,
        tx: TransactionRequest,
    ) -> Result<TransactionReceipt, StrategistError> {
        let client = self.get_request_provider().await?;

        // Convert string address to Address type
        let addr = Address::from_str(from_address)?;

        // Start impersonating the account
        self.impersonate_account(from_address).await?;

        // Set the from field in the transaction
        let impersonated_tx = tx.from(addr);

        // Send the transaction
        let tx_response = client
            .send_transaction(impersonated_tx)
            .await?
            .get_receipt()
            .await?;

        // Stop impersonating the account
        self.stop_impersonating_account(from_address).await?;

        Ok(tx_response)
    }
}

// Implement the trait for any type that implements EvmBaseClient
// This makes the impersonation methods available on any EvmBaseClient
impl<T> AnvilImpersonationClient for T where T: EvmBaseClient {}
