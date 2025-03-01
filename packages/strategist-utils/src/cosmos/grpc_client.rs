use tonic::{async_trait, transport::Channel};

use crate::common::error::StrategistError;

use super::signing_client::SigningClient;

#[async_trait]
pub trait GrpcSigningClient {
    async fn get_grpc_channel(&self) -> Result<Channel, StrategistError>;
    async fn get_signing_client(&self) -> Result<SigningClient, StrategistError>;
}
