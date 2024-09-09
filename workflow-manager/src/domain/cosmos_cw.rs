use std::{
    collections::HashMap,
    fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cosmos_grpc_client::{
    cosmos_sdk_proto::cosmwasm::wasm::v1::{MsgInstantiateContract2, QueryCodeRequest},
    cosmrs::bip32::secp256k1::sha2::{digest::Update, Digest, Sha256},
    Decimal, GrpcClient, ProstMsgNameToAny, Wallet,
};
use serde_json::to_vec;
use thiserror::Error;
use valence_authorization_utils::domain::ExternalDomain;
use valence_processor::msg::PolytoneContracts;

use crate::{
    account::{AccountType, InstantiateAccountData},
    bridges::PolytoneSingleChainInfo,
    config::{ChainInfo, ConfigError, CONFIG},
    service::{ServiceConfig, ServiceError},
    MAIN_CHAIN,
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
    ConfigError(#[from] ConfigError),

    #[error(transparent)]
    ServiceError(#[from] ServiceError),
}

pub struct CosmosCosmwasmConnector {
    is_main_chain: bool,
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
            is_main_chain: chain_info.name == MAIN_CHAIN,
            wallet,
            code_ids: code_ids.clone(),
            _chain_name: chain_info.name.clone(),
        })
    }
}

#[async_trait]
impl Connector for CosmosCosmwasmConnector {
    async fn get_address(
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

        let checksum = self.get_checksum(code_id).await?;

        // TODO: generate a unique salt per workflow and per contract by adding timestamp
        let since_the_epoch = SystemTime::now()
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

    async fn get_address_bridge(
        &mut self,
        sender_addr: &str,
        main_chain: &str,
        sender_chain: &str,
        receiving_chain: &str,
    ) -> ConnectorResult<String> {
        // Get the checksum of the code id
        let code_id = *self
            .code_ids
            .get("polytone_proxy")
            .context(format!("Code id not found for: {}", "polytone_proxy"))
            .map_err(CosmosCosmwasmError::Error)?;
        let receiving_chain_bridge_info =
            self.get_bridge_info(main_chain, sender_chain, receiving_chain)?;

        let checksum = self.get_checksum(code_id).await?;

        let salt = Sha256::new()
            .chain(receiving_chain_bridge_info.connection_id)
            .chain(receiving_chain_bridge_info.other_note_port)
            .chain(sender_addr)
            .finalize()
            .to_vec();

        let addr = self
            .wallet
            .client
            .proto_query::<QueryBuildAddressRequest, QueryBuildAddressResponse>(
                QueryBuildAddressRequest {
                    code_hash: hex::encode(checksum.clone()),
                    creator_address: receiving_chain_bridge_info.voice_addr,
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

        Ok(addr)
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

        // TODO: Add workflow id to the label
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

        // TODO: Add workflow id to the label
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

    async fn instantiate_authorization(
        &mut self,
        workflow_id: u64,
        salt: Vec<u8>,
        processor_addr: String,
        external_domains: Vec<ExternalDomain>,
    ) -> ConnectorResult<()> {
        // If we are not on the main chain, we error out, should not happen
        if !self.is_main_chain {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Authorization contract can only be instantiated on the main chain"
            ))
            .into());
        }

        let code_id = *self
            .code_ids
            .get("authorization")
            .context(format!("Code id not found for: {}", "authorization"))
            .map_err(CosmosCosmwasmError::Error)?;

        let msg = to_vec(&valence_authorization::msg::InstantiateMsg {
            owner: self.wallet.account_address.clone(),
            sub_owners: vec![],
            processor: processor_addr,
            external_domains,
        })
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin: self.wallet.account_address.clone(),
            code_id,
            label: format!("valence-authorization-{}", workflow_id),
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

    async fn instantiate_processor(
        &mut self,
        workflow_id: u64,
        salt: Vec<u8>,
        admin: String,
        polytone_addr: Option<PolytoneContracts>,
    ) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get("processor")
            .context(format!("Code id not found for: {}", "processor"))
            .map_err(CosmosCosmwasmError::Error)?;

        let msg = to_vec(&valence_processor::msg::InstantiateMsg {
            owner: admin.clone(),
            authorization_contract: admin.clone(),
            polytone_contracts: polytone_addr,
        })
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin,
            code_id,
            label: format!("valence-processor-{}", workflow_id),
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

// Helpers
impl CosmosCosmwasmConnector {
    pub fn get_bridge_info(
        &self,
        main_chain: &str,
        sender_chain: &str,
        receive_chain: &str,
    ) -> Result<PolytoneSingleChainInfo, CosmosCosmwasmError> {
        let info = if main_chain == sender_chain {
            CONFIG
                .get_bridge_info(sender_chain, receive_chain)?
                .get_polytone_info()
        } else if main_chain == receive_chain {
            CONFIG
                .get_bridge_info(receive_chain, sender_chain)?
                .get_polytone_info()
        } else {
            return Err(anyhow!(
                "Failed to get brdige info, none of the provded chains is the main chain"
            )
            .into());
        };

        info.get(receive_chain)
            .context(format!("Bridge info not found for: {}", receive_chain))
            .map_err(CosmosCosmwasmError::Error)
            .cloned()
    }
    pub async fn get_checksum(&mut self, code_id: u64) -> Result<Vec<u8>, CosmosCosmwasmError> {
        let req = QueryCodeRequest { code_id };
        let code_res = self
            .wallet
            .client
            .clients
            .wasm
            .code(req)
            .await
            .context(format!("Code request failed for: {}", code_id))?;

        Ok(code_res
            .into_inner()
            .code_info
            .context(format!(
                "Failed to parse the response of code id: {}",
                code_id
            ))?
            .data_hash)
    }
}
