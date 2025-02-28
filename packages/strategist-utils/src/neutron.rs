use std::str::FromStr;

use crate::{
    common::{
        error::StrategistError, signing_client::SigningClient, transaction::TransactionResponse,
    },
    cosmos::base_client::BaseClient,
};
use async_trait::async_trait;

use cosmos_sdk_proto::cosmos::bank::v1beta1::QueryBalanceResponse;
use cosmrs::{
    bank::MsgSend,
    cosmwasm::MsgExecuteContract,
    proto::{
        cosmos::{
            bank::v1beta1::QueryBalanceRequest,
            base::tendermint::v1beta1::{
                service_client::ServiceClient as TendermintServiceClient, GetLatestBlockRequest,
            },
        },
        cosmwasm::wasm::v1::QuerySmartContractStateRequest,
    },
    tx::Msg,
    AccountId, Coin,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tonic::{transport::Channel, Request};

const CHAIN_PREFIX: &str = "neutron";
const CHAIN_ID: &str = "neutron-1";

pub struct NeutronClient {
    grpc_url: String,
    mnemonic: String,
    chain_id: String,
}

impl NeutronClient {
    pub fn new(rpc_url: &str, rpc_port: &str, mnemonic: &str, chain_id: &str) -> Self {
        Self {
            grpc_url: format!("{rpc_url}:{rpc_port}"),
            mnemonic: mnemonic.to_string(),
            chain_id: chain_id.to_string(),
        }
    }

    pub async fn get_grpc_channel(&self) -> Result<Channel, StrategistError> {
        Ok(Channel::from_shared(self.grpc_url.clone())
            .map_err(|_| StrategistError::ClientError("failed to build channel".to_string()))?
            .connect()
            .await
            .unwrap())
    }

    pub async fn get_signing_client(&self) -> Result<SigningClient, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        SigningClient::from_mnemonic(channel, &self.mnemonic, CHAIN_PREFIX, &self.chain_id).await
    }
}

#[async_trait]
impl BaseClient for NeutronClient {
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

        let height = u64::try_from(block_header.height)
            .map_err(|_| StrategistError::ParseError("failed to get height".to_string()))?;

        Ok(height)
    }

    async fn query_balance(&self, address: &str, denom: &str) -> Result<u128, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut bank_client =
            cosmrs::proto::cosmos::bank::v1beta1::query_client::QueryClient::new(channel);

        let request = QueryBalanceRequest {
            address: address.to_string(),
            denom: denom.to_string(),
        };

        let response: QueryBalanceResponse = bank_client
            .balance(Request::new(request))
            .await?
            .into_inner();

        let coin = response
            .balance
            .ok_or_else(|| StrategistError::QueryError("No balance returned".to_string()))?;

        let amount = coin
            .amount
            .parse::<u128>()
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        Ok(amount)
    }

    async fn query_contract_state<T: DeserializeOwned>(
        &self,
        contract_address: &str,
        query_data: Value,
    ) -> Result<T, StrategistError> {
        let channel = self.get_grpc_channel().await?;

        let mut wasm_client =
            cosmrs::proto::cosmwasm::wasm::v1::query_client::QueryClient::new(channel);

        let bin_query = serde_json::to_vec(&query_data)
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        let request = QuerySmartContractStateRequest {
            address: contract_address.to_string(),
            query_data: bin_query,
        };

        let response = wasm_client
            .smart_contract_state(Request::new(request))
            .await?
            .into_inner();

        let parsed: T = serde_json::from_slice(&response.data)
            .map_err(|e| StrategistError::ParseError(e.to_string()))?;

        Ok(parsed)
    }

    async fn transfer(
        &self,
        to: &str,
        amount: u128,
        denom: &str,
        options: Option<String>,
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
        .to_any()
        .unwrap();

        let raw_tx = signing_client.create_tx(transfer_msg).await?;

        let mut client =
            cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient::new(channel);

        let broadcast_tx_response = client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }

    async fn execute_wasm<T: Serialize + Send + 'static>(
        &self,
        contract: &str,
        msg: T,
        funds: Vec<Coin>,
    ) -> Result<TransactionResponse, StrategistError> {
        let signing_client = self.get_signing_client().await?;
        let channel = self.get_grpc_channel().await?;

        let msg_bytes = serde_json::to_vec(&msg).unwrap();
        let wasm_tx = MsgExecuteContract {
            sender: signing_client.address.clone(),
            contract: AccountId::from_str(contract)?,
            msg: msg_bytes,
            funds,
        }
        .to_any()
        .unwrap();

        let raw_tx = signing_client.create_tx(wasm_tx).await?;

        let mut client =
            cosmos_sdk_proto::cosmos::tx::v1beta1::service_client::ServiceClient::new(channel);

        let broadcast_tx_response = client.broadcast_tx(raw_tx).await?.into_inner();

        match broadcast_tx_response.tx_response {
            Some(tx_response) => Ok(TransactionResponse::try_from(tx_response)?),
            None => Err(StrategistError::TransactionError("failed".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;

    const LOCAL_GRPC_URL: &str = "http://127.0.0.1";
    const LOCAL_GRPC_PORT: &str = "42977";
    const LOCAL_MNEMONIC: &str = "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry";
    const LOCAL_SIGNER_ADDR: &str = "neutron1hj5fveer5cjtn4wd6wstzugjfdxzl0xpznmsky";
    const LOCAL_ALT_ADDR: &str = "neutron1kljf09rj77uxeu5lye7muejx6ajsu55cuw2mws";
    const LOCAL_CHAIN_ID: &str = "localneutron-1";
    const LOCAL_PROCESSOR_ADDR: &str =
        "neutron1tdwtzvhep8nwxyy4pyry520lncshum9wshyvpv2w393nmf75jxjsyfq4ll";

    // update during dev to a real one for mainnet testing
    const _CHAIN_ID: &str = "neutron-1";
    const _GRPC_URL: &str = "-";
    const _GRPC_PORT: &str = "-";
    const _NEUTRON_DAO_ADDR: &str =
        "neutron1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrstdxvff";
    const _MNEMONIC: &str = "-";

    #[tokio::test]
    async fn test_latest_block_height() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
        );

        let block_height = client
            .latest_block_height()
            .await
            .expect("Failed to get latest block height");

        println!("latest block height: {}", block_height);
    }

    #[tokio::test]
    async fn test_query_balance() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
        );
        let balance = client
            .query_balance(LOCAL_SIGNER_ADDR, "untrn")
            .await
            .unwrap();

        assert_ne!(balance, 0);
    }

    #[tokio::test]
    async fn test_query_contract_state() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
        );

        let query_data = serde_json::json!({
          "config": {}
        });

        let state: Value = client
            .query_contract_state(LOCAL_PROCESSOR_ADDR, query_data)
            .await
            .unwrap();

        println!("state: {:?}", state);
    }

    #[tokio::test]
    async fn test_transfer() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
        );

        let pre_transfer_balance = client.query_balance(LOCAL_ALT_ADDR, "untrn").await.unwrap();

        client
            .transfer(LOCAL_ALT_ADDR, 100_000, "untrn", None)
            .await
            .unwrap();

        sleep(Duration::from_secs(3)).await;

        let post_transfer_balance = client.query_balance(LOCAL_ALT_ADDR, "untrn").await.unwrap();

        assert_eq!(pre_transfer_balance + 100_000, post_transfer_balance);
    }

    #[tokio::test]
    async fn test_execute_wasm() {
        let client = NeutronClient::new(
            LOCAL_GRPC_URL,
            LOCAL_GRPC_PORT,
            LOCAL_MNEMONIC,
            LOCAL_CHAIN_ID,
        );

        let processor_tick_msg = valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
            valence_processor_utils::msg::PermissionlessMsg::Tick {},
        );

        let resp = client
            .execute_wasm(LOCAL_PROCESSOR_ADDR, processor_tick_msg, vec![])
            .await
            .unwrap();

        println!("response: {:?}", resp);
    }
}
