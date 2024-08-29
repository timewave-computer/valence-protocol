use std::{
    collections::HashMap,
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use async_trait::async_trait;
use cosmos_grpc_client::{
    cosmos_sdk_proto::cosmwasm::wasm::v1::{MsgInstantiateContract2, QueryCodeRequest},
    cosmrs::bip32::secp256k1::sha2::{digest::Update, Digest, Sha256},
    Decimal, GrpcClient, ProstMsgNameToAny, Wallet,
};
use serde_json::to_vec;
use thiserror::Error;

use crate::{
    account::{AccountType, InstantiateAccountData},
    config::ChainInfo,
    service::{ServiceConfig, ServiceError},
};

use super::{Connector, ConnectorResult};

const MNEMONIC: &str = "crazy into this wheel interest enroll basket feed fashion leave feed depth wish throw rack language comic hand family shield toss leisure repair kite";

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryBuildAddressRequest {
    #[prost(string, tag = "1")]
    pub code_hash: String,
    #[prost(string, tag = "2")]
    pub creator_address: String,
    #[prost(string, tag = "3")]
    pub salt: String,
    // #[prost(string, tag = "2")]
    // pub creator_address: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryBuildAddressResponse {
    #[prost(string, tag = "1")]
    pub address: String,
}

#[derive(Error, Debug)]
pub enum CosmosCosmwasmError {
    #[error(transparent)]
    Error(#[from] anyhow::Error),

    #[error(transparent)]
    GrpcError(#[from] cosmos_grpc_client::StdError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),
}

pub struct CosmosCosmwasmConnector {
    wallet: Wallet,
    code_ids: HashMap<String, u64>,
    _chain_name: String,
}

impl fmt::Debug for CosmosCosmwasmConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosCosmwasmConnector")
            .field("wallet", &self.wallet)
            .finish_non_exhaustive()
    }
}

impl CosmosCosmwasmConnector {
    pub async fn new(
        chain_info: &ChainInfo,
        code_ids: &HashMap<String, u64>,
    ) -> Result<Self, CosmosCosmwasmError> {
        let grpc = GrpcClient::new(&chain_info.grpc).await.context(format!(
            "Failed to create new client for: {}",
            chain_info.name
        ))?;

        let gas_price = Decimal::from_str(&chain_info.gas_price)?;
        let gas_adj = Decimal::from_str("1.5")?;

        let wallet = Wallet::from_seed_phrase(
            grpc,
            MNEMONIC,
            chain_info.prefix.clone(),
            chain_info.coin_type,
            0,
            gas_price,
            gas_adj,
            &chain_info.gas_denom,
        )
        .await
        .context(format!(
            "Failed to create new wallet for {}",
            chain_info.name
        ))?;

        Ok(CosmosCosmwasmConnector {
            wallet,
            code_ids: code_ids.clone(),
            _chain_name: chain_info.name.clone(),
        })
    }
}

#[async_trait]
impl Connector for CosmosCosmwasmConnector {
    async fn predict_address(
        &mut self,
        id: &u64,
        contract_name: &str,
        extra_salt: &str,
    ) -> ConnectorResult<(String, Vec<u8>)> {
        // Get the checksum of the code id
        let code_id = *self
            .code_ids
            .get(contract_name)
            .context(format!("Code id not found for: {}", contract_name))
            .map_err(CosmosCosmwasmError::Error)?;

        let req = QueryCodeRequest { code_id };
        let code_res = self
            .wallet
            .client
            .clients
            .wasm
            .code(req)
            .await
            .context(format!("Code request failed for: {}", code_id))
            .map_err(CosmosCosmwasmError::Error)?;

        let checksum = code_res
            .into_inner()
            .code_info
            .context(format!(
                "Failed to parse the response of code id: {}",
                code_id
            ))
            .map_err(CosmosCosmwasmError::Error)?
            .data_hash;

        // TODO: generate a unique salt per workflow and per contract by adding timestamp
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();

        let salt = Sha256::new()
            .chain(contract_name)
            .chain(id.to_string())
            .chain(extra_salt)
            .chain(since_the_epoch.to_string())
            .finalize()
            .to_vec();

        let addr = self
            .wallet
            .client
            .proto_query::<QueryBuildAddressRequest, QueryBuildAddressResponse>(
                QueryBuildAddressRequest {
                    code_hash: hex::encode(checksum.clone()),
                    creator_address: self.wallet.account_address.clone(),
                    salt: hex::encode(salt.clone()),
                },
                "/cosmwasm.wasm.v1.Query/BuildAddress",
            )
            .await
            .context(format!(
                "Failed to query the instantiate2 address: {:?}",
                checksum
            ))
            .map_err(CosmosCosmwasmError::Error)?
            .address;

        Ok((addr, salt.to_vec()))
    }

    async fn instantiate_account(&mut self, data: &InstantiateAccountData) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get(&data.info.ty.to_string())
            .context(format!("Code id not found for: {}", data.info.ty))
            .map_err(CosmosCosmwasmError::Error)?;

        // TODO: change the admin to authorization
        let msg: Vec<u8> = match &data.info.ty {
            AccountType::Base { admin } => to_vec(&valence_base_account::msg::InstantiateMsg {
                admin: admin
                    .clone()
                    .unwrap_or_else(|| self.wallet.account_address.to_string()),
                approved_services: data.approved_services.clone(),
            })
            .map_err(CosmosCosmwasmError::SerdeJsonError)?,
            AccountType::Addr { .. } => return Ok(()),
        };

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin: self.wallet.account_address.clone(),
            code_id,
            label: format!("account-{}", data.id),
            msg,
            funds: vec![],
            salt: data.salt.clone(),
            fix_msg: false,
        }
        .build_any();

        self.wallet
            .simulate_tx(vec![m])
            // .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync) // TODO: change once we ready
            .await
            .map(|_| ())
            .context("Failed to broadcast the TX")
            .map_err(|e| CosmosCosmwasmError::Error(e).into())
    }

    async fn instantiate_service(
        &mut self,
        service_id: u64,
        service_config: &ServiceConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get(&service_config.to_string())
            .context(format!("Code id not found for: {}", service_config))
            .map_err(CosmosCosmwasmError::Error)?;

        // TODO: change the admin to authorization
        let msg = service_config
            .get_instantiate_msg(
                self.wallet.account_address.clone(),
                self.wallet.account_address.clone(),
            )
            .map_err(CosmosCosmwasmError::ServiceError)?;

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin: self.wallet.account_address.clone(),
            code_id,
            label: format!("service-{}-{}", service_config, service_id),
            msg,
            funds: vec![],
            salt: salt.clone(),
            fix_msg: false,
        }
        .build_any();

        self.wallet
            .simulate_tx(vec![m])
            // .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync) // TODO: change once we ready
            .await
            .map(|_| ())
            .map_err(|e| CosmosCosmwasmError::Error(e).into())
    }
}
