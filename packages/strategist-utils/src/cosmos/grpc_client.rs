use tonic::{async_trait, transport::Channel};

use crate::common::error::StrategistError;

use super::signing_client::SigningClient;

#[async_trait]
pub trait GrpcSigningClient {
    fn grpc_url(&self) -> String;
    fn mnemonic(&self) -> String;
    fn chain_prefix(&self) -> String;
    fn chain_id(&self) -> String;
    fn chain_denom(&self) -> String;

    async fn get_grpc_channel(&self) -> Result<Channel, StrategistError> {
        Ok(Channel::from_shared(self.grpc_url())
            .map_err(|_| StrategistError::ClientError("failed to build channel".to_string()))?
            .connect()
            .await
            .unwrap())
    }

    async fn get_signing_client(&self) -> Result<SigningClient, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        SigningClient::from_mnemonic(
            channel,
            &self.mnemonic(),
            &self.chain_prefix(),
            &self.chain_id(),
        )
        .await
    }
}
