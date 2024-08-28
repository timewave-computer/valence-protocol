use std::{
    collections::HashMap,
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

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
    error::ManagerResult,
    service::{ServiceConfig, ServiceError},
};

use super::Connector;

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

pub type CosmosCosmwasmResult<T> = Result<T, CosmosCosmwasmError>;

#[derive(Error, Debug)]
pub enum CosmosCosmwasmError {
    #[error("cosmos_grpc_client Error: {0}")]
    GrpcError(#[from] cosmos_grpc_client::StdError),

    #[error("serde_json Error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),

    #[error("Chain not found for: {0}")]
    ChainInfoNotFound(String),

    #[error("Code ids not found for: {0}")]
    CodeIdsNotFound(String),

    #[error("Failed to query the code id: {0}")]
    FailedQueryCodeId(u64),

    #[error("Failed to parse the response of code id: {0}")]
    FailedParseResCodeId(u64),

    #[error("Failed to create new client for: {0} | {1}")]
    FailedNewClient(String, String),

    #[error("Failed to create new wallet for: {0} | {1}")]
    FailedNewWalletInstance(String, String),

    #[error("Failed to query the instantiate2 address: {0}")]
    FailedQueryAddress2(anyhow::Error),

    #[error("Failed to broadcast the TX: {0}")]
    FailedBroadcastTx(anyhow::Error),
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
    ) -> ManagerResult<Self> {
        let grpc = GrpcClient::new(&chain_info.grpc).await.map_err(|e| {
            CosmosCosmwasmError::FailedNewClient(chain_info.name.to_string(), e.to_string())
        })?;

        let gas_price =
            Decimal::from_str(&chain_info.gas_price).map_err(CosmosCosmwasmError::GrpcError)?;
        let gas_adj = Decimal::from_str("1.5").map_err(CosmosCosmwasmError::GrpcError)?;

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
        .map_err(|e| {
            CosmosCosmwasmError::FailedNewWalletInstance(chain_info.name.to_string(), e.to_string())
        })?;

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
    ) -> ManagerResult<(String, Vec<u8>)> {
        // Get the checksum of the code id
        let code_id =
            *self
                .code_ids
                .get(contract_name)
                .ok_or(CosmosCosmwasmError::CodeIdsNotFound(
                    contract_name.to_string(),
                ))?;

        let req = QueryCodeRequest { code_id };
        let code_res = self
            .wallet
            .client
            .clients
            .wasm
            .code(req)
            .await
            .map_err(|_| CosmosCosmwasmError::FailedQueryCodeId(code_id))?;

        let checksum = code_res
            .into_inner()
            .code_info
            .ok_or(CosmosCosmwasmError::FailedParseResCodeId(code_id))?
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
            .map_err(CosmosCosmwasmError::FailedQueryAddress2)?
            .address;

        Ok((addr, salt.to_vec()))
    }

    async fn instantiate_account(&mut self, data: &InstantiateAccountData) -> ManagerResult<()> {
        let code_id = *self.code_ids.get(&data.info.ty.to_string()).ok_or(
            CosmosCosmwasmError::CodeIdsNotFound(data.info.ty.to_string()),
        )?;

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
            .map_err(|e| CosmosCosmwasmError::FailedBroadcastTx(e).into())
    }

    async fn instantiate_service(
        &mut self,
        service_id: u64,
        service_config: &ServiceConfig,
        salt: Vec<u8>,
    ) -> ManagerResult<()> {
        let code_id = *self.code_ids.get(&service_config.to_string()).ok_or(
            CosmosCosmwasmError::CodeIdsNotFound(service_config.to_string()),
        )?;

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
            .map_err(|e| CosmosCosmwasmError::FailedBroadcastTx(e).into())
    }
}
