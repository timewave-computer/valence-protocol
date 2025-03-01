use std::str::FromStr;

use async_trait::async_trait;
use cosmos_sdk_proto::cosmos::{
    bank::v1beta1::{QueryBalanceRequest, QueryBalanceResponse},
    base::abci::v1beta1::TxResponse,
    tx::v1beta1::GetTxRequest,
};

use cosmrs::proto::cosmos::base::tendermint::v1beta1::{
    service_client::ServiceClient as TendermintServiceClient, GetLatestBlockRequest,
};
use cosmrs::{bank::MsgSend, tx::Msg, AccountId, Coin};
use tonic::Request;

use crate::common::{error::StrategistError, transaction::TransactionResponse};

use super::grpc_client::GrpcSigningClient;

/// base client trait with default implementations for cosmos-sdk based clients.
///
/// for chains which are somehow unique in their common module implementations,
/// these function definitions can be overridden to match that of the chain.
#[async_trait]
pub trait BaseClient: GrpcSigningClient {
    async fn transfer(
        &self,
        to: &str,
        amount: u128,
        denom: &str,
        fee_denom: &str,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;
        let channel = self.get_grpc_channel().await?;

        let amount = Coin {
            denom: denom.parse()?,
            amount,
        };

        let transfer_msg = MsgSend {
            from_address: signing_client.address.clone(),
            to_address: AccountId::from_str(to)?,
            amount: vec![amount],
        }
        .to_any()?;

        let raw_tx = signing_client
            .create_tx(transfer_msg, fee_denom, 500_000, 500_000u64, None)
            .await?;

        let mut grpc_client =
            cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }

    async fn latest_block_height(&self) -> Result<u64, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut tendermint_client = TendermintServiceClient::new(channel);

        let response = tendermint_client
            .get_latest_block(GetLatestBlockRequest {})
            .await?
            .into_inner();

        let sdk_block = response
            .sdk_block
            .ok_or_else(|| StrategistError::QueryError("no block in response".to_string()))?;

        let block_header = sdk_block
            .header
            .ok_or_else(|| StrategistError::QueryError("no header in sdk_block".to_string()))?;

        let height = u64::try_from(block_header.height)?;

        Ok(height)
    }

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client =
            cosmrs::proto::cosmos::bank::v1beta1::query_client::QueryClient::new(channel);

        let request = QueryBalanceRequest {
            address: address.to_string(),
            denom: denom.to_string(),
        };

        let response: QueryBalanceResponse = grpc_client
            .balance(Request::new(request))
            .await?
            .into_inner();

        let coin = response
            .balance
            .ok_or_else(|| StrategistError::QueryError("No balance returned".to_string()))?;

        let amount = coin.amount.parse::<u128>()?;

        Ok(amount)
    }

    // expected utils
    async fn poll_for_tx(&self, tx_hash: &str) -> Result<TxResponse, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client =
            cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient::new(channel);

        let request = GetTxRequest {
            hash: tx_hash.to_string(),
        };

        // using tokio for timing utils instead of system to not block the entire thread.
        //
        // for 5 seconds it will repeatedly fire tx polling requests to the node.
        // if 100ms turns out to hit the node too hard, increase it. maybe this can be
        // passed in as an arg.
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
        for _ in 1..50 {
            interval.tick().await;
            let rx = grpc_client.get_tx(request.clone()).await;
            match rx {
                Ok(response) => {
                    let r = response.into_inner();
                    println!("{:?}", r.tx_response);
                    if let Some(tx_response) = r.tx_response {
                        return Ok(tx_response);
                    }
                }
                Err(tonic_status) => match tonic_status.code() {
                    // if tx code not found, continue polling
                    tonic::Code::NotFound => {
                        continue;
                    }
                    // otherwise return the error
                    _ => break,
                },
            };
        }

        Err(StrategistError::QueryError(
            "failed to confirm tx".to_string(),
        ))
    }
}
