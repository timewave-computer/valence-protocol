use alloy::transports::http::reqwest;
use cosmos_sdk_proto::cosmos::tx::v1beta1::{SimulateRequest, SimulateResponse};
use cosmrs::{
    tx::{BodyBuilder, Fee, SignDoc, SignerInfo},
    Any, Coin,
};
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
    fn gas_price(&self) -> f64;
    fn gas_adjustment(&self) -> f64;

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
            .unwrap_or(200_000);

        let adjusted_gas_limit = gas_used as f64 * self.gas_adjustment();

        let coin_amount = Coin {
            denom: self.chain_denom().parse()?,
            amount: (adjusted_gas_limit * self.gas_price()) as u128 + 1,
        };
        let gas_limit = adjusted_gas_limit as u64;

        Ok(Fee::from_amount_and_gas(coin_amount, gas_limit))
    }

    /// simulates a transaction with the given message.
    async fn simulate_tx(&self, msg: Any) -> Result<SimulateResponse, StrategistError> {
        let channel = self.get_grpc_channel().await?;
        let signer = self.get_signing_client().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let tx_body = BodyBuilder::new().msg(msg).finish();
        let auth_info = SignerInfo::single_direct(Some(signer.public_key), signer.sequence)
            .auth_info(cosmrs::tx::Fee::from_amount_and_gas(
                Coin {
                    denom: self.chain_denom().parse()?,
                    amount: 0,
                },
                0u64,
            ));

        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &signer.chain_id.parse()?,
            signer.account_number,
        )?;

        let tx_raw = sign_doc.sign(&signer.signing_key)?;

        #[allow(deprecated)]
        let request = SimulateRequest {
            // tx is deprecated so always None
            tx: None,
            tx_bytes: tx_raw.to_bytes()?,
        };

        let sim_response = grpc_client.simulate(request).await?.into_inner();

        Ok(sim_response)
    }

    /// fetches the chain-registry config for the given chain and denom and returns
    /// the average gas price for the chain denom.
    async fn query_chain_gas_config(chain: &str, denom: &str) -> Result<f64, StrategistError> {
        let chain_registry_url = format!(
            "https://raw.githubusercontent.com/cosmos/chain-registry/master/{chain}/chain.json"
        );

        let response = reqwest::get(chain_registry_url).await?;

        let config: serde_json::Value = response.json().await?;

        let fee_tokens = config["fees"]["fee_tokens"]
            .as_array()
            .ok_or_else(|| StrategistError::QueryError("failed to get fee_tokens".to_string()))?;

        let native_fee = fee_tokens
            .iter()
            .find(|entry| entry["denom"] == denom)
            .unwrap();

        let average_gas_price = native_fee["average_gas_price"].as_f64().ok_or_else(|| {
            StrategistError::QueryError("failed to get chain denom average gas price".to_string())
        })?;

        Ok(average_gas_price)
    }
}
