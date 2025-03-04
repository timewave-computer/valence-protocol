use cosmos_sdk_proto::cosmos::tx::v1beta1::{SimulateRequest, SimulateResponse};
use cosmrs::{tx::Fee, Any, Coin};
use tonic::{async_trait, transport::Channel};

use crate::common::error::StrategistError;

use super::{signing_client::SigningClient, CosmosServiceClient};

/// grpc signing client trait to enable transaction signing and grpc channel opening.
/// implementing this trait is a prerequisite for any clients dealing with cosmos-sdk
/// base or wasm funcionalities.
#[async_trait]
pub trait GrpcSigningClient {
    fn grpc_url(&self) -> String;
    fn mnemonic(&self) -> String;
    fn chain_prefix(&self) -> String;
    fn chain_id(&self) -> String;
    fn chain_denom(&self) -> String;

    /// opens and returns a grpc channel associated with the grpc url of the
    /// implementing client
    async fn get_grpc_channel(&self) -> Result<Channel, StrategistError> {
        let channel = Channel::from_shared(self.grpc_url())
            .map_err(|_| StrategistError::ClientError("failed to build channel".to_string()))?
            .connect()
            .await?;

        Ok(channel)
    }

    /// returns a signing client associated with the implementing client config
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

    fn get_tx_fee(&self, simulation_response: SimulateResponse) -> Result<Fee, StrategistError> {
        let gas_used = simulation_response
            .gas_info
            .map(|info| info.gas_used)
            .unwrap_or(200_000) as u128;

        let adjusted_gas_limit = (gas_used as f64 * 1.5) as u128;
        println!("adjusted gas limit: {:?}", adjusted_gas_limit);

        Ok(Fee::from_amount_and_gas(
            Coin {
                denom: self.chain_denom().parse()?,
                amount: adjusted_gas_limit + 1,
            },
            adjusted_gas_limit as u64 - 1,
        ))
    }

    async fn simulate_tx(&self, msg: Any) -> Result<SimulateResponse, StrategistError> {
        let channel = self.get_grpc_channel().await?;
        let signer = self.get_signing_client().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let tx_body = cosmrs::tx::BodyBuilder::new().msg(msg).finish();
        let auth_info =
            cosmrs::tx::SignerInfo::single_direct(Some(signer.public_key), signer.sequence)
                .auth_info(cosmrs::tx::Fee::from_amount_and_gas(
                    Coin {
                        denom: self.chain_denom().parse()?,
                        amount: 0,
                    },
                    0u64,
                ));

        let sign_doc = cosmrs::tx::SignDoc::new(
            &tx_body,
            &auth_info,
            &signer.chain_id.parse()?,
            signer.account_number,
        )?;

        let tx_raw = sign_doc.sign(&signer.signing_key)?;

        let request = SimulateRequest {
            // tx is deprecated so always None
            tx: None,
            tx_bytes: tx_raw.to_bytes()?,
        };

        let sim_response = grpc_client.simulate(request).await?.into_inner();

        Ok(sim_response)
    }
}
