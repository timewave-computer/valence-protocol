use std::str::FromStr;

use async_trait::async_trait;
use cosmos_sdk_proto::cosmos::{
    auth::v1beta1::{
        ModuleAccount, QueryModuleAccountByNameRequest, QueryModuleAccountByNameResponse,
    },
    bank::v1beta1::{QueryBalanceRequest, QueryBalanceResponse},
    base::{abci::v1beta1::TxResponse, tendermint::v1beta1::Header},
    tx::v1beta1::GetTxRequest,
};

use cosmrs::{
    bank::MsgSend,
    rpc::{Client, HttpClient},
    tendermint::block::Height,
    tx::Msg,
    AccountId, Coin, Denom,
};
use cosmrs::{
    proto::cosmos::base::tendermint::v1beta1::{
        service_client::ServiceClient as TendermintServiceClient, GetLatestBlockRequest,
    },
    Any,
};
use log::{info, warn};
use prost::Message;
use tonic::Request;

use crate::common::{error::StrategistError, transaction::TransactionResponse};

use super::{
    grpc_client::GrpcSigningClient, proto_timestamp::ProtoTimestamp, AuthQueryClient,
    BankQueryClient, CosmosServiceClient,
};

/// base client trait with default implementations for cosmos-sdk based clients.
///
/// for chains which are somehow unique in their common module implementations,
/// these function definitions can be overridden to match the custom chain logic.
#[async_trait]
pub trait BaseClient: GrpcSigningClient {
    fn proto_coin(denom: &str, amt: u128) -> Result<Coin, StrategistError> {
        let proto_denom: Denom = denom.parse()?;
        Ok(Coin {
            denom: proto_denom,
            amount: amt,
        })
    }

    async fn transfer(
        &self,
        to: &str,
        amount: u128,
        denom: &str,
        memo: Option<&str>,
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

        let simulation_response = self.simulate_tx(transfer_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(transfer_msg, fee, memo).await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
    }

    async fn latest_block_header(&self) -> Result<Header, StrategistError> {
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

        Ok(block_header)
    }

    async fn block_results(
        &self,
        rpc_addr: &str,
        height: u32,
    ) -> Result<cosmrs::rpc::endpoint::block_results::Response, StrategistError> {
        let client = HttpClient::new(rpc_addr)?;

        let height = Height::from(height);

        let results = client.block_results(height).await?;

        Ok(results)
    }

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = BankQueryClient::new(channel);

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

    async fn query_module_account(&self, name: &str) -> Result<ModuleAccount, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = AuthQueryClient::new(channel);

        let request = QueryModuleAccountByNameRequest {
            name: name.to_string(),
        };

        let response: QueryModuleAccountByNameResponse = grpc_client
            .module_account_by_name(Request::new(request))
            .await?
            .into_inner();

        let module_account_any = response
            .account
            .ok_or_else(|| StrategistError::QueryError("No module account returned".to_string()))?;

        // Decode the bytes into a ModuleAccount
        let module_account = ModuleAccount::decode(&*module_account_any.value).map_err(|e| {
            StrategistError::QueryError(format!("Failed to decode ModuleAccount: {}", e))
        })?;

        Ok(module_account)
    }

    // expected utils
    async fn poll_for_tx(&self, tx_hash: &str) -> Result<TxResponse, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

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

    async fn poll_until_expected_balance(
        &self,
        address: &str,
        denom: &str,
        min_amount: u128,
        interval_sec: u64,
        max_attempts: u32,
    ) -> Result<u128, StrategistError> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec));

        info!("Polling {address} balance to exceed {denom}{min_amount}");

        for attempt in 1..max_attempts + 1 {
            interval.tick().await;

            match self.query_balance(address, denom).await {
                Ok(balance) => {
                    if balance >= min_amount {
                        return Ok(balance);
                    }
                    info!(
                        "Balance polling attempt {attempt}/{max_attempts}: current={balance}, target={min_amount}"
                    );
                }
                Err(e) => {
                    warn!(
                        "Balance polling attempt {attempt}/{max_attempts} failed: {:?}",
                        e
                    );
                }
            }
        }

        Err(StrategistError::QueryError(format!(
            "Balance did not exceed {min_amount}{denom} after {max_attempts} attempts"
        )))
    }

    async fn ibc_transfer(
        &self,
        to: String,
        denom: String,
        amount: String,
        channel_id: String,
        timeout_seconds: u64,
        memo: Option<String>,
    ) -> Result<TransactionResponse, StrategistError> {
        // first we query the latest block header to respect the chain time for timeouts
        let latest_block_header = self.latest_block_header().await?;

        let mut current_time = ProtoTimestamp::try_from(latest_block_header)?;

        current_time.extend_by_seconds(timeout_seconds)?;

        let timeout_nanos = current_time.to_nanos()?;

        let signing_client = self.get_signing_client().await?;

        let ibc_transfer_msg = ibc::apps::transfer::types::proto::transfer::v1::MsgTransfer {
            source_port: "transfer".to_string(),
            source_channel: channel_id,
            token: Some(cosmos_sdk_proto::cosmos::base::v1beta1::Coin { denom, amount }),
            sender: signing_client.address.to_string(),
            receiver: to,
            timeout_height: None,
            timeout_timestamp: timeout_nanos,
            memo: memo.unwrap_or_default(),
        };

        let any_msg = Any::from_msg(&ibc_transfer_msg)?;

        let simulation_response = self.simulate_tx(any_msg.clone()).await?;
        let fee = self.get_tx_fee(simulation_response)?;

        let raw_tx = signing_client.create_tx(any_msg, fee, None).await?;

        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        TransactionResponse::try_from(broadcast_tx_response.tx_response)
    }
}
