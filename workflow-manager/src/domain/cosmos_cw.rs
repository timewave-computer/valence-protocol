use std::{
    collections::HashMap,
    fmt,
    str::{from_utf8, FromStr},
};

use crate::{
    account::{AccountType, InstantiateAccountData},
    bridge::PolytoneSingleChainInfo,
    config::{ChainInfo, ConfigError, CONFIG},
    service::{ServiceConfig, ServiceError},
    MAIN_CHAIN, NEUTRON_DOMAIN,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cosmos_grpc_client::{
    cosmos_sdk_proto::{
        cosmos::tx::v1beta1::GetTxRequest,
        cosmwasm::wasm::v1::{
            MsgExecuteContract, MsgInstantiateContract2, QueryCodeRequest,
            QuerySmartContractStateRequest,
        },
    },
    cosmrs::bip32::secp256k1::sha2::{digest::Update, Digest, Sha256},
    BroadcastMode, Decimal, GrpcClient, ProstMsgNameToAny, Wallet,
};
use cosmwasm_std::from_json;
use serde_json::to_vec;
use thiserror::Error;
use tokio::time::sleep;

use super::{Connector, ConnectorResult, POLYTONE_TIMEOUT};

const MNEMONIC: &str = "margin moon alcohol assume tube bullet long cook edit delay boat camp stone coyote gather design aisle comfort width sound innocent long dumb jungle";

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

    #[error(transparent)]
    CosmwasmStdError(#[from] cosmwasm_std::StdError),
}

pub struct CosmosCosmwasmConnector {
    is_main_chain: bool,
    wallet: Wallet,
    code_ids: HashMap<String, u64>,
    chain_name: String,
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
            321,
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
            chain_name: chain_info.name.clone(),
        })
    }
}

#[async_trait]
impl Connector for CosmosCosmwasmConnector {
    async fn reserve_workflow_id(&mut self) -> ConnectorResult<u64> {
        if self.chain_name != NEUTRON_DOMAIN.get_chain_name() {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Should only be implemented on neutron connector"
            ))
            .into());
        }
        let registry_addr = CONFIG.get_registry_addr();

        // Execute a message to reserve the workflow id
        let msg = to_vec(&valence_workflow_registry_utils::ExecuteMsg::ReserveId {})
            .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgExecuteContract {
            sender: self.wallet.account_address.clone(),
            contract: registry_addr,
            msg,
            funds: vec![],
        }
        .build_any();

        let tx_hash = self
            .wallet
            // .simulate_tx(vec![m])
            .broadcast_tx(vec![m], None, None, BroadcastMode::Sync)
            .await
            .map_err(CosmosCosmwasmError::Error)?
            .tx_response
            .unwrap()
            .txhash;

        // We rely on the tx hash above to be on chain before we can query to get its events.
        // so we are sleeping here for a good 15 seconds to make sure we have the tx on chain.
        // TODO: We can have a retry logic here to sleep for 5 seconds at a time until we get the tx on chain.
        sleep(std::time::Duration::from_secs(15)).await;

        let res = self
            .wallet
            .client
            .clients
            .tx
            .get_tx(GetTxRequest { hash: tx_hash })
            .await
            .context("'reserve_workflow_id' Failed to query the chain for TX")
            .map_err(CosmosCosmwasmError::Error)?
            .into_inner()
            .tx_response
            .context("'reserve_workflow_id' Failed to get `tx_response`")
            .map_err(CosmosCosmwasmError::Error)?
            .events
            .iter()
            .find_map(|e| {
                if e.r#type == "wasm"
                    && e.attributes[0].key == "_contract_address"
                    && e.attributes[0].value == CONFIG.get_registry_addr()
                    && e.attributes[1].key == "method"
                    && e.attributes[1].value == "reserve_id"
                {
                    Some(e.attributes[2].value.clone())
                } else {
                    None
                }
            })
            .context("'reserve_workflow_id' Failed to find the event with the id")
            .map_err(CosmosCosmwasmError::Error)?;

        Ok(from_utf8(&res)
            .context("'reserve_workflow_id' Failed to convert bytes to string")
            .map_err(CosmosCosmwasmError::Error)?
            .parse::<u64>()
            .context("'reserve_workflow_id' Failed to parse string to u64")
            .map_err(CosmosCosmwasmError::Error)?)
    }

    async fn get_address(
        &mut self,
        id: u64,
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

        let salt = Sha256::new()
            .chain(contract_name)
            .chain(id.to_string())
            .chain(extra_salt)
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
            // .broadcast_tx(vec![m], None, None, BroadcastMode::Sync) // TODO: change once we ready
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

        let msg = to_vec(&valence_authorization_utils::msg::InstantiateMsg {
            owner: self.wallet.account_address.clone(),
            sub_owners: vec![],
            processor: processor_addr,
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

    async fn change_authorization_owner(
        &mut self,
        authorization_addr: String,
        owner: String,
    ) -> ConnectorResult<()> {
        let msg = to_vec(
            &valence_authorization_utils::msg::ExecuteMsg::UpdateOwnership(
                cw_ownable::Action::TransferOwnership {
                    new_owner: owner,
                    expiry: None,
                },
            ),
        )
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgExecuteContract {
            sender: self.wallet.account_address.clone(),
            contract: authorization_addr,
            msg,
            funds: vec![],
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
        polytone_addr: Option<valence_processor_utils::msg::PolytoneContracts>,
    ) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get("processor")
            .context(format!("Code id not found for: {}", "processor"))
            .map_err(CosmosCosmwasmError::Error)?;

        let msg = to_vec(&valence_processor_utils::msg::InstantiateMsg {
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

    // TODO: Currently its only working for polytone, we will need to support other bridges at some point
    async fn add_external_domain(
        &mut self,
        main_domain: &str,
        domain: &str,
        authorrization_addr: String,
        processor_addr: String,
        processor_bridge_account_addr: String,
    ) -> ConnectorResult<()> {
        if !self.is_main_chain {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Adding external domain is only possible on main domain in authorization contract"
            ))
            .into());
        }

        let bridge = self.get_bridge_info(main_domain, main_domain, domain)?;

        let external_domain = valence_authorization_utils::msg::ExternalDomainInfo {
            name: domain.to_string(),
            execution_environment:
                valence_authorization_utils::domain::ExecutionEnvironment::CosmWasm,
            connector: valence_authorization_utils::msg::Connector::PolytoneNote {
                address: bridge.note_addr,
                timeout_seconds: POLYTONE_TIMEOUT,
            },

            processor: processor_addr,
            callback_proxy: valence_authorization_utils::msg::CallbackProxy::PolytoneProxy(
                processor_bridge_account_addr,
            ),
        };

        let msg = to_vec(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                valence_authorization_utils::msg::PermissionedMsg::AddExternalDomains {
                    external_domains: vec![external_domain],
                },
            ),
        )
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgExecuteContract {
            sender: self.wallet.account_address.clone(),
            contract: authorrization_addr,
            msg,
            funds: vec![],
        }
        .build_any();

        self.wallet
            .simulate_tx(vec![m])
            // .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync) // TODO: change once we ready
            .await
            .map(|_| ())
            .map_err(|e| CosmosCosmwasmError::Error(e).into())
    }

    async fn instantiate_processor_bridge_account(
        &mut self,
        processor_addr: String,
        retry: u8,
    ) -> ConnectorResult<()> {
        // We check if we should retry or not,
        let should_retry = self
            ._should_retry_processor_bridge_account_creation(processor_addr.clone(), 5, 60)
            .await?;

        if should_retry {
            if retry == 0 {
                return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                    "'instantiate_processor_bridge_account', max retry reached"
                ))
                .into());
            } else {
                let msg = to_vec(
                    &valence_processor_utils::msg::ExecuteMsg::PermissionlessAction(
                        valence_processor_utils::msg::PermissionlessMsg::RetryBridgeCreation {},
                    ),
                )
                .map_err(CosmosCosmwasmError::SerdeJsonError)?;

                let m = MsgExecuteContract {
                    sender: self.wallet.account_address.clone(),
                    contract: processor_addr.clone(),
                    msg,
                    funds: vec![],
                }
                .build_any();

                self.wallet
                    .simulate_tx(vec![m])
                    // .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync) // TODO: change once we ready
                    .await
                    .map(|_| ())
                    .map_err(CosmosCosmwasmError::Error)?;

                return self
                    .instantiate_processor_bridge_account(processor_addr, retry - 1)
                    .await;
            }
        }

        Ok(())
    }

    async fn instantiate_authorization_bridge_account(
        &mut self,
        authorization_addr: String,
        domain: String,
        retry: u8,
    ) -> ConnectorResult<()> {
        // We check if we should retry or not,
        let should_retry = self
            ._should_retry_authorization_bridge_account_creation(
                authorization_addr.clone(),
                domain.clone(),
                5,
                60,
            )
            .await?;

        if should_retry {
            if retry == 0 {
                return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                    "'instantiate_authorization_bridge_account', max retry reached"
                ))
                .into());
            } else {
                let msg = to_vec(
                    &valence_authorization_utils::msg::ExecuteMsg::PermissionlessAction(
                        valence_authorization_utils::msg::PermissionlessMsg::RetryBridgeCreation {
                            domain_name: domain.clone(),
                        },
                    ),
                )
                .map_err(CosmosCosmwasmError::SerdeJsonError)?;

                let m = MsgExecuteContract {
                    sender: self.wallet.account_address.clone(),
                    contract: authorization_addr.clone(),
                    msg,
                    funds: vec![],
                }
                .build_any();

                self.wallet
                    .simulate_tx(vec![m])
                    // .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync) // TODO: change once we ready
                    .await
                    .map(|_| ())
                    .map_err(CosmosCosmwasmError::Error)?;

                return self
                    .instantiate_authorization_bridge_account(authorization_addr, domain, retry - 1)
                    .await;
            }
        }

        Ok(())
    }
}

// Helpers
impl CosmosCosmwasmConnector {
    /// Here we check if we should retry or not the bridge account creation
    /// It will error we have reached our maximum retry amount
    /// It will send a response otherwise
    pub async fn _should_retry_processor_bridge_account_creation(
        &mut self,
        processor_addr: String,
        retry_amount: u64,
        sleep_duration: u64,
    ) -> Result<bool, CosmosCosmwasmError> {
        let query_data = to_vec(&valence_processor_utils::msg::QueryMsg::Config {})
            .map_err(CosmosCosmwasmError::SerdeJsonError)?;
        let config_req = QuerySmartContractStateRequest {
            address: processor_addr.clone(),
            query_data,
        };

        let config_res = from_json::<valence_processor_utils::processor::Config>(
            self.wallet
                .client
                .clients
                .wasm
                .smart_contract_state(config_req)
                .await
                .context("'_should_retry_processor_bridge_account_creation' Failed to query the processor")
                .map_err(CosmosCosmwasmError::Error)?
                .into_inner()
                .data,
        )
        .map_err(CosmosCosmwasmError::CosmwasmStdError)?;

        let state = match config_res.processor_domain {
            valence_processor_utils::processor::ProcessorDomain::Main => {
                return Err(anyhow!("'_should_retry_processor_bridge_account_creation' Processor domain is main, should be external").into())
            }
            valence_processor_utils::processor::ProcessorDomain::External(polytone) => {
                polytone.proxy_on_main_domain_state
            }
        };

        match state {
            valence_authorization_utils::domain::PolytoneProxyState::TimedOut => {
                // if timeouted, we should retry the account creation
                Ok(true)
            }
            valence_authorization_utils::domain::PolytoneProxyState::PendingResponse => {
                // Still pending, but we reached our maximum retry amount
                if retry_amount == 0 {
                    return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                        "'_should_retry_processor_bridge_account_creation' Max retry reached"
                    )));
                }
                // Still pending and have retries left to do, we sleep, and then retry the check
                sleep(std::time::Duration::from_secs(sleep_duration)).await;
                Box::pin(self._should_retry_processor_bridge_account_creation(
                    processor_addr,
                    retry_amount - 1,
                    sleep_duration,
                ))
                .await
            }
            valence_authorization_utils::domain::PolytoneProxyState::Created => {
                // Account was created, so we don't need to do anything anymore
                Ok(false)
            }
            valence_authorization_utils::domain::PolytoneProxyState::UnexpectedError(err) => {
                // We got an unexpected error, we should retry the account creation
                Err(CosmosCosmwasmError::Error(anyhow::anyhow!(format!(
                    "'_should_retry_processor_bridge_account_creation' UnexpectedError: {}",
                    err
                ))))
            }
        }
    }

    pub async fn _should_retry_authorization_bridge_account_creation(
        &mut self,
        authorization_addr: String,
        domain: String,
        retry_amount: u64,
        sleep_duration: u64,
    ) -> Result<bool, CosmosCosmwasmError> {
        let query_data = to_vec(
            &valence_authorization_utils::msg::QueryMsg::ExternalDomain {
                name: domain.clone(),
            },
        )
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;
        let req = QuerySmartContractStateRequest {
            address: authorization_addr.clone(),
            query_data,
        };

        let res = from_json::<valence_authorization_utils::domain::ExternalDomain>(
            self.wallet
                .client
                .clients
                .wasm
                .smart_contract_state(req)
                .await
                .context("'_should_retry_authorization_bridge_account_creation' Failed to query the authorization")
                .map_err(CosmosCosmwasmError::Error)?
                .into_inner()
                .data,
        )
        .map_err(CosmosCosmwasmError::CosmwasmStdError)?;

        let state = match res.clone().connector {
            valence_authorization_utils::domain::Connector::PolytoneNote { state, .. } => state,
        };

        match state {
            valence_authorization_utils::domain::PolytoneProxyState::TimedOut => {
                // if timeouted, we should retry the account creation
                Ok(true)
            }
            valence_authorization_utils::domain::PolytoneProxyState::PendingResponse => {
                // Still pending, but we reached our maximum retry amount
                if retry_amount == 0 {
                    return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                        "'_should_retry_authorization_bridge_account_creation' Max retry reached"
                    )));
                }
                // Still pending and have retries left to do, we sleep, and then retry the check
                sleep(std::time::Duration::from_secs(sleep_duration)).await;
                Box::pin(self._should_retry_authorization_bridge_account_creation(
                    authorization_addr,
                    domain,
                    retry_amount - 1,
                    sleep_duration,
                ))
                .await
            }
            valence_authorization_utils::domain::PolytoneProxyState::Created => {
                // Account was created, so we don't need to do anything anymore
                Ok(false)
            }
            valence_authorization_utils::domain::PolytoneProxyState::UnexpectedError(err) => {
                // We got an unexpected error, we should retry the account creation
                Err(CosmosCosmwasmError::Error(anyhow::anyhow!(format!(
                    "'_should_retry_authorization_bridge_account_creation' UnexpectedError: {}",
                    err
                ))))
            }
        }
    }

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
