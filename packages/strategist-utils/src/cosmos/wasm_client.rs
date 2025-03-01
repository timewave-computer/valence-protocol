use std::str::FromStr;

use async_trait::async_trait;
use cosmrs::Coin;
use serde::{de::DeserializeOwned, Serialize};

use crate::common::{error::StrategistError, transaction::TransactionResponse};
use tonic::Request;

use super::grpc_client::GrpcSigningClient;

use cosmrs::{
    cosmwasm::MsgExecuteContract, proto::cosmwasm::wasm::v1::QuerySmartContractStateRequest,
    tx::Msg, AccountId,
};

#[async_trait]
pub trait WasmClient: GrpcSigningClient {
    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: (impl Serialize + Send),
    ) -> Result<T, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut grpc_client =
            cosmrs::proto::cosmwasm::wasm::v1::query_client::QueryClient::new(channel);

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

    async fn execute_wasm<T: Serialize + Send + 'static>(
        &self,
        contract: &str,
        msg: T,
        fee_denom: &str,
        funds: Vec<Coin>,
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

        let raw_tx = signing_client
            .create_tx(wasm_tx, fee_denom, 500_000, 500_000u64, None)
            .await?;

        let mut grpc_client =
            cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient::new(channel);

        let broadcast_tx_response = grpc_client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }
}
