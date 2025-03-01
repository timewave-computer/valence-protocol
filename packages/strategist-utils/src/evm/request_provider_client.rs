use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    transports::http::reqwest,
};
use alloy_signer_local::PrivateKeySigner;
use tonic::async_trait;

use crate::common::error::StrategistError;

use super::base_client::CustomProvider;

#[async_trait]
pub trait RequestProviderClient {
    fn rpc_url(&self) -> String;
    fn signer(&self) -> PrivateKeySigner;

    async fn get_request_provider(&self) -> Result<CustomProvider, StrategistError> {
        let url: reqwest::Url = match self.rpc_url().parse() {
            Ok(resp) => resp,
            Err(e) => return Err(StrategistError::ParseError(e.to_string())),
        };

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
