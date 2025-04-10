use std::str::FromStr;

use async_trait::async_trait;
use cosmrs::{tx::Fee, Coin};
use serde::{de::DeserializeOwned, Serialize};

use crate::common::{error::StrategistError, transaction::TransactionResponse};
use tonic::Request;

use super::{grpc_client::GrpcSigningClient, CosmosServiceClient, WasmQueryClient};

use cosmrs::{
    cosmwasm::MsgExecuteContract, proto::cosmwasm::wasm::v1::QuerySmartContractStateRequest,
    tx::Msg, AccountId,
};

/// wasm funcionality trait with default implementations for cosmos-sdk based clients.
///
/// for chains which are somehow unique in their wasm module implementations,
/// these function definitions can be overridden to match that of the chain.
#[async_trait]
pub trait WasmClient: GrpcSigningClient {
    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: (impl Serialize + Send),
    ) -> Result<T, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client = WasmQueryClient::new(channel);

        let bin_query = serde_json::to_vec(&query_data)?;

        let request = QuerySmartContractStateRequest {
            address: contract_address.to_string(),
            query_data: bin_query,
        };

        let response = grpc_client
            .smart_contract_state(Request::new(request))
            .await?
            .into_inner();

        let parsed: T = serde_json::from_slice(&response.data)?;

        Ok(parsed)
    }

    async fn execute_wasm(
        &self,
        contract: &str,
        msg: (impl Serialize + Send),
        funds: Vec<Coin>,
        fees: Option<Fee>,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;
        let channel = self.get_grpc_channel().await?;

        let msg_bytes = serde_json::to_vec(&msg)?;

        let wasm_tx = MsgExecuteContract {
            sender: signing_client.address.clone(),
            contract: AccountId::from_str(contract)?,
            msg: msg_bytes,
            funds,
        }
        .to_any()?;

        let simulation_response = self.simulate_tx(wasm_tx.clone()).await?;

        // if no fees were specified we simulate the tx and use the estimated fee
        let tx_fee = fees.unwrap_or(self.get_tx_fee(simulation_response)?);

        let raw_tx = signing_client.create_tx(wasm_tx, tx_fee, None).await?;

        let mut grpc_client = CosmosServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }
}
