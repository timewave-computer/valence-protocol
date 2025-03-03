use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    transports::http::reqwest,
};
use alloy_signer_local::PrivateKeySigner;
use tonic::async_trait;

use crate::common::error::StrategistError;

use super::base_client::CustomProvider;

/// trait for evm-based clients to enable signing and request provider functionality.
/// each implementation must provide getters for the rpc url and signer which are used
/// to build the provider and sign transactions.
#[async_trait]
pub trait RequestProviderClient {
    fn rpc_url(&self) -> String;
    fn signer(&self) -> PrivateKeySigner;

    async fn get_request_provider(&self) -> Result<CustomProvider, StrategistError> {
        let url: reqwest::Url = self
            .rpc_url()
            .parse()
            .map_err(|_| StrategistError::ParseError("failed to parse url".to_string()))?;

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(url);

        Ok(provider)
    }

    async fn get_provider_accounts(&self) -> Result<Vec<Address>, StrategistError> {
        let provider = self.get_request_provider().await?;
        let accounts = provider.get_accounts().await?;
        Ok(accounts)
    }
}
