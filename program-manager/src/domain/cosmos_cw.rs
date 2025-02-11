use std::{
    collections::HashMap,
    fmt,
    str::{from_utf8, FromStr},
};

use crate::{
    account::{AccountType, InstantiateAccountData},
    bridge::PolytoneSingleChainInfo,
    config::{ChainInfo, ConfigError, GLOBAL_CONFIG},
    helpers::{addr_canonicalize, addr_humanize},
    library::{LibraryConfig, LibraryError},
    program_config::ProgramConfig,
    NEUTRON_CHAIN,
};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use cosmos_grpc_client::{
    cosmos_sdk_proto::{
        cosmos::tx::v1beta1::{GetTxRequest, GetTxResponse},
        cosmwasm::wasm::v1::{
            MsgExecuteContract, MsgInstantiateContract2, QueryCodeRequest,
            QueryContractInfoRequest, QuerySmartContractStateRequest,
        },
    },
    cosmrs::bip32::secp256k1::sha2::{digest::Update, Digest, Sha256, Sha512},
    BroadcastMode, Decimal, GrpcClient, ProstMsgNameToAny, Wallet,
};
use cosmwasm_std::{from_json, instantiate2_address, to_json_binary};
use futures::future::BoxFuture;
use serde_json::to_vec;
use strum::VariantNames;
use thiserror::Error;
use tokio::time::sleep;
use valence_authorization_utils::authorization::AuthorizationInfo;

use super::{Connector, ConnectorResult, POLYTONE_TIMEOUT};

// const MNEMONIC: &str = "margin moon alcohol assume tube bullet long cook edit delay boat camp stone coyote gather design aisle comfort width sound innocent long dumb jungle";
// acc0 in local-ic
// const MNEMONIC: &str = "decorate bright ozone fork gallery riot bus exhaust worth way bone indoor calm squirrel merry zero scheme cotton until shop any excess stage laundry";
// acc1 in local-ic
const MNEMONIC: &str = "across army acoustic hurt help sad turkey switch popular fade purse obvious session tuition file asset cover agree number motor pupil slim hundred busy";

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
    LibraryError(#[from] LibraryError),

    #[error(transparent)]
    CosmwasmStdError(#[from] cosmwasm_std::StdError),
}

pub struct CosmosCosmwasmConnector {
    is_main_chain: bool,
    wallet: Wallet,
    code_ids: HashMap<String, u64>,
    chain_name: String,
    prefix: String,
}

impl fmt::Debug for CosmosCosmwasmConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CosmosCosmwasmConnector")
            .field("wallet", &self.wallet)
            .finish_non_exhaustive()
    }
}

impl CosmosCosmwasmConnector {
    pub async fn new(chain_name: &str) -> Result<Self, CosmosCosmwasmError> {
        let gc = GLOBAL_CONFIG.lock().await;
        let chain_info: &ChainInfo = gc.get_chain_info(chain_name)?;
        let code_ids: &HashMap<String, u64> = gc.get_code_ids(chain_name)?;

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
            is_main_chain: chain_info.name == *NEUTRON_CHAIN,
            wallet,
            code_ids: code_ids.clone(),
            chain_name: chain_info.name.clone(),
            prefix: chain_info.prefix.clone(),
        })
    }
}

#[async_trait]
impl Connector for CosmosCosmwasmConnector {
    async fn reserve_program_id(&mut self) -> ConnectorResult<u64> {
        if self.chain_name != *NEUTRON_CHAIN {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Should only be implemented on neutron connector"
            ))
            .into());
        }
        let registry_addr = GLOBAL_CONFIG.lock().await.get_registry_addr();

        // Execute a message to reserve the program id
        let msg = to_vec(&valence_program_registry_utils::ExecuteMsg::ReserveId {})
            .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgExecuteContract {
            sender: self.wallet.account_address.clone(),
            contract: registry_addr.clone(),
            msg,
            funds: vec![],
        }
        .build_any();

        // We rely on the tx hash above to be on chain before we can query to get its events.
        let res = self.broadcast_tx(m, "reserve_program_id").await?;

        let res = res
            .tx_response
            .context("'reserve_program_id' Failed to get `tx_response`")
            .map_err(CosmosCosmwasmError::Error)?
            .events
            .iter()
            .find_map(|e| {
                if e.r#type == "wasm"
                    && e.attributes[0].key == "_contract_address"
                    && e.attributes[0].value == registry_addr
                    && e.attributes[1].key == "method"
                    && e.attributes[1].value == "reserve_id"
                {
                    Some(e.attributes[2].value.clone())
                } else {
                    None
                }
            })
            .context("'reserve_program_id' Failed to find the event with the id")
            .map_err(CosmosCosmwasmError::Error)?;

        Ok(from_utf8(&res)
            .context("'reserve_program_id' Failed to convert bytes to string")
            .map_err(CosmosCosmwasmError::Error)?
            .parse::<u64>()
            .context("'reserve_program_id' Failed to parse string to u64")
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
            .chain(GLOBAL_CONFIG.lock().await.get_registry_addr())
            .finalize()
            .to_vec();

        let addr_canonical = instantiate2_address(
            &checksum,
            &addr_canonicalize(&self.prefix, self.wallet.account_address.as_str()).unwrap(),
            &salt,
        )
        .context("Failed to instantiate2 address")
        .map_err(CosmosCosmwasmError::Error)?;

        let addr =
            addr_humanize(&self.prefix, &addr_canonical).map_err(CosmosCosmwasmError::Error)?;

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

        let receiving_chain_bridge_info = self
            .get_bridge_info(main_chain, sender_chain, receiving_chain)
            .await?;

        let checksum = self.get_checksum(code_id).await?;

        let salt = Sha512::new()
            .chain_update(receiving_chain_bridge_info.connection_id.as_bytes())
            .chain_update(receiving_chain_bridge_info.other_note_port.as_bytes())
            .chain_update(sender_addr.as_bytes())
            .finalize()
            .to_vec();

        let addr_canonical = instantiate2_address(
            &checksum,
            &addr_canonicalize(
                &self.prefix,
                receiving_chain_bridge_info.voice_addr.as_str(),
            )
            .unwrap(),
            &salt,
        )
        .context("Failed to instantiate2 address")
        .map_err(CosmosCosmwasmError::Error)?;

        let addr =
            addr_humanize(&self.prefix, &addr_canonical).map_err(CosmosCosmwasmError::Error)?;

        Ok(addr)
    }

    async fn instantiate_account(
        &mut self,
        program_id: u64,
        processor_addr: String,
        data: &InstantiateAccountData,
    ) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get(&data.info.ty.to_string())
            .context(format!("Code id not found for: {}", data.info.ty))
            .map_err(CosmosCosmwasmError::Error)?;

        let msg: Vec<u8> = match &data.info.ty {
            AccountType::Base { admin } => to_vec(&valence_account_utils::msg::InstantiateMsg {
                admin: admin.clone().unwrap_or_else(|| processor_addr.clone()),
                approved_libraries: data.approved_libraries.clone(),
            })
            .map_err(CosmosCosmwasmError::SerdeJsonError)?,
            AccountType::Addr { .. } => return Ok(()),
        };

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin: processor_addr,
            code_id,
            label: format!("program-{}|account-{}", program_id, data.id),
            msg,
            funds: vec![],
            salt: data.salt.clone(),
            fix_msg: false,
        }
        .build_any();

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "instantiate_account").await?;

        Ok(())
    }

    async fn instantiate_library(
        &mut self,
        program_id: u64,
        processor_addr: String,
        library_id: u64,
        library_config: LibraryConfig,
        salt: Vec<u8>,
    ) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get(&library_config.to_string())
            .context(format!("Code id not found for: {}", library_config))
            .map_err(CosmosCosmwasmError::Error)?;

        let msg = library_config
            .get_instantiate_msg(processor_addr.clone(), processor_addr.clone())
            .map_err(CosmosCosmwasmError::LibraryError)?;

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin: processor_addr,
            code_id,
            label: format!(
                "program-{}|library-{}-{}",
                program_id, library_config, library_id
            ),
            msg,
            funds: vec![],
            salt: salt.clone(),
            fix_msg: false,
        }
        .build_any();

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "instantiate_library").await?;

        Ok(())
    }

    async fn instantiate_authorization(
        &mut self,
        program_id: u64,
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
            .get("valence_authorization")
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
            label: format!("valence-authorization-{}", program_id),
            msg,
            funds: vec![],
            salt: salt.clone(),
            fix_msg: false,
        }
        .build_any();

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "instantiate_authorization").await?;

        Ok(())
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

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "change_authorization_owner").await?;

        Ok(())
    }

    async fn instantiate_processor(
        &mut self,
        program_id: u64,
        salt: Vec<u8>,
        admin: String,
        authorization: String,
        polytone_config: Option<valence_processor_utils::msg::PolytoneContracts>,
    ) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get("valence_processor")
            .context(format!("Code id not found for: {}", "processor"))
            .map_err(CosmosCosmwasmError::Error)?;

        let msg = to_vec(&valence_processor_utils::msg::InstantiateMsg {
            authorization_contract: authorization,
            polytone_contracts: polytone_config,
        })
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgInstantiateContract2 {
            sender: self.wallet.account_address.clone(),
            admin,
            code_id,
            label: format!("valence-processor-{}", program_id),
            msg,
            funds: vec![],
            salt: salt.clone(),
            fix_msg: false,
        }
        .build_any();

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "instantiate_processor").await?;

        Ok(())
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

        let bridge = self
            .get_bridge_info(main_domain, domain, main_domain)
            .await?;

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

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "add_external_domain").await?;

        Ok(())
    }

    async fn add_authorizations(
        &mut self,
        authorization_addr: String,
        authorizations: Vec<AuthorizationInfo>,
    ) -> ConnectorResult<()> {
        if !self.is_main_chain {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Adding authorizations is only possible on main domain in authorization contract"
            ))
            .into());
        }

        let msg = to_vec(
            &valence_authorization_utils::msg::ExecuteMsg::PermissionedAction(
                valence_authorization_utils::msg::PermissionedMsg::CreateAuthorizations {
                    authorizations,
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

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "add_authorizations").await?;

        Ok(())
    }

    async fn instantiate_processor_bridge_account(
        &mut self,
        processor_addr: String,
        retry: u8,
    ) -> ConnectorResult<()> {
        // We check if we should retry or not,
        let should_retry = self
            .should_retry_processor_bridge_account_creation(processor_addr.clone(), 5, 60)
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

                // Broadcast the tx and wait for it to finalize (or error)
                self.broadcast_tx(m, "instantiate_processor_bridge_account")
                    .await?;

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
            .should_retry_authorization_bridge_account_creation(
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

                // Broadcast the tx and wait for it to finalize (or error)
                self.broadcast_tx(m, "instantiate_authorization_bridge_account")
                    .await?;

                return self
                    .instantiate_authorization_bridge_account(authorization_addr, domain, retry - 1)
                    .await;
            }
        }

        Ok(())
    }

    async fn query_program_registry(
        &mut self,
        id: u64,
    ) -> ConnectorResult<valence_program_registry_utils::ProgramResponse> {
        if self.chain_name != NEUTRON_CHAIN {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "program registry only exists on neutron chain"
            ))
            .into());
        }

        // Query for program config for an idea
        let query_data = to_vec(&valence_program_registry_utils::QueryMsg::GetConfig { id })
            .map_err(CosmosCosmwasmError::SerdeJsonError)?;
        let config_req = QuerySmartContractStateRequest {
            address: GLOBAL_CONFIG.lock().await.get_registry_addr().clone(),
            query_data,
        };

        let config_res = from_json::<valence_program_registry_utils::ProgramResponse>(
            self.wallet
                .client
                .clients
                .wasm
                .smart_contract_state(config_req)
                .await
                .context("'query_program_registry' Failed to query program registry")
                .map_err(CosmosCosmwasmError::Error)?
                .into_inner()
                .data,
        )
        .map_err(CosmosCosmwasmError::CosmwasmStdError)?;

        Ok(config_res)
    }

    async fn verify_account(&mut self, account_addr: String) -> ConnectorResult<()> {
        let contract_name = self.get_contract_name_by_address(account_addr).await?;

        // Loop over account types and see if any matches the contract name of the code id
        // error if it doesn't match, else return ()
        Ok(AccountType::VARIANTS
            .iter()
            .find(|x| x.to_string() == contract_name)
            .context("'verify_account' Code id doesn't match any account type")
            .map(|_| ())
            .map_err(CosmosCosmwasmError::Error)?)
    }

    async fn verify_library(&mut self, library_addr: Option<String>) -> ConnectorResult<()> {
        let library_addr = library_addr
            .context("'verify_library' Library address is empty")
            .map_err(CosmosCosmwasmError::Error)?;

        let contract_name = self.get_contract_name_by_address(library_addr).await?;

        Ok(LibraryConfig::VARIANTS
            .iter()
            .find(|x| x.to_string() == contract_name)
            .context("'verify_account' Code id doesn't match any account type")
            .map(|_| ())
            .map_err(CosmosCosmwasmError::Error)?)
    }

    async fn verify_processor(&mut self, processor_addr: String) -> ConnectorResult<()> {
        let contract_name = self.get_contract_name_by_address(processor_addr).await?;

        // Make sure the code id is of name processor
        if contract_name != "valence_processor" {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "'verify_processor' Code id isn't of processor code id"
            ))
            .into());
        }

        Ok(())
    }

    async fn verify_bridge_account(&mut self, bridge_addr: String) -> ConnectorResult<()> {
        // If the address has a code id, it means it was instantiated.
        self.get_code_id_of_addr(bridge_addr)
            .await
            .map(|_| ())
            .map_err(|e| e.into())
    }

    async fn verify_authorization_addr(&mut self, addr: String) -> ConnectorResult<()> {
        let code_id = *self
            .code_ids
            .get("valence_authorization")
            .context(format!("Code id not found for: {}", "authorization"))
            .map_err(CosmosCosmwasmError::Error)?;

        let code_id_on_chain = self.get_code_id_of_addr(addr).await?;

        if code_id_on_chain != code_id {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "'verify_authorization_addr' Code id doesn't match authorization"
            ))
            .into());
        }

        Ok(())
    }

    async fn save_program_config(&mut self, mut config: ProgramConfig) -> ConnectorResult<()> {
        if self.chain_name != *NEUTRON_CHAIN {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Should only be implemented on neutron connector"
            ))
            .into());
        }

        for library in config.libraries.values_mut() {
            if library.addr.is_none() {
                return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                    "Before saving program config each library must have an address"
                ))
                .into());
            }

            library.config = LibraryConfig::None;
        }

        let registry_addr = GLOBAL_CONFIG.lock().await.get_registry_addr();

        let program_binary =
            to_json_binary(&config).map_err(CosmosCosmwasmError::CosmwasmStdError)?;

        let msg = to_vec(&valence_program_registry_utils::ExecuteMsg::SaveProgram {
            id: config.id,
            program_config: program_binary,
        })
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgExecuteContract {
            sender: self.wallet.account_address.clone(),
            contract: registry_addr,
            msg,
            funds: vec![],
        }
        .build_any();

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "save_program_config").await?;

        Ok(())
    }

    async fn update_program_config(&mut self, config: ProgramConfig) -> ConnectorResult<()> {
        if self.chain_name != *NEUTRON_CHAIN {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Should only be implemented on neutron connector"
            ))
            .into());
        }

        for library in config.libraries.values() {
            if library.addr.is_none() {
                return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                    "Before saving program config each library must have an address"
                ))
                .into());
            }
        }

        let registry_addr = GLOBAL_CONFIG.lock().await.get_registry_addr();

        let program_binary =
            to_json_binary(&config).map_err(CosmosCosmwasmError::CosmwasmStdError)?;

        let msg = to_vec(&valence_program_registry_utils::ExecuteMsg::UpdateProgram {
            id: config.id,
            program_config: program_binary,
        })
        .map_err(CosmosCosmwasmError::SerdeJsonError)?;

        let m = MsgExecuteContract {
            sender: self.wallet.account_address.clone(),
            contract: registry_addr,
            msg,
            funds: vec![],
        }
        .build_any();

        // Broadcast the tx and wait for it to finalize (or error)
        self.broadcast_tx(m, "update_workflow_config").await?;

        Ok(())
    }

    async fn get_program_config(&mut self, id: u64) -> ConnectorResult<ProgramConfig> {
        if self.chain_name != *NEUTRON_CHAIN {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Should only be implemented on neutron connector"
            ))
            .into());
        }

        let registry_addr = GLOBAL_CONFIG.lock().await.get_registry_addr();

        let query = QuerySmartContractStateRequest {
            address: registry_addr,
            query_data: to_vec(&valence_program_registry_utils::QueryMsg::GetConfig { id })
                .map_err(CosmosCosmwasmError::SerdeJsonError)?,
        };

        let res = from_json::<valence_program_registry_utils::ProgramResponse>(
            &self
                .wallet
                .client
                .clients
                .wasm
                .smart_contract_state(query)
                .await
                .context("Failed to query the workflow config from registry")
                .map_err(CosmosCosmwasmError::Error)?
                .into_inner()
                .data,
        )
        .map_err(CosmosCosmwasmError::CosmwasmStdError)?;

        Ok(from_json::<ProgramConfig>(&res.program_config)
            .map_err(CosmosCosmwasmError::CosmwasmStdError)?)
    }
}

// Helpers
impl CosmosCosmwasmConnector {
    async fn broadcast_tx(
        &mut self,
        msg: prost_types::Any,
        err_id: &str,
    ) -> Result<GetTxResponse, CosmosCosmwasmError> {
        let res = self
            .wallet
            .broadcast_tx(vec![msg], None, None, BroadcastMode::Sync)
            .await
            .map_err(CosmosCosmwasmError::Error)?
            .tx_response
            .context(format!("'{err_id}' failed to get tx_response"))
            .map_err(CosmosCosmwasmError::Error)?;

        if res.code != 0 {
            return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                "Failed {err_id}: {res:?}",
            )));
        }

        // wait for the tx to be on chain
        self.query_tx_hash(res.txhash, 15).await
    }

    /// we retry every second, so retry here means how many seconds we should wait for the tx to appear.
    /// If the tx is on chain we will return early, so retry is the MAX amount of seconds we will wait.
    fn query_tx_hash(
        &mut self,
        hash: String,
        retry: u64,
    ) -> BoxFuture<Result<GetTxResponse, CosmosCosmwasmError>> {
        Box::pin(async move {
            if retry == 0 {
                return Err(CosmosCosmwasmError::Error(anyhow::anyhow!(
                    "'query_tx_hash' failed, Max retry reached"
                )));
            };

            sleep(std::time::Duration::from_secs(1)).await;

            let res = self
                .wallet
                .client
                .clients
                .tx
                .get_tx(GetTxRequest { hash: hash.clone() })
                .await;

            match res {
                Ok(r) => Ok(r.into_inner()),
                Err(_) => self.query_tx_hash(hash, retry - 1).await,
            }
        })
    }

    /// Here we check if we should retry or not the bridge account creation
    /// It will error we have reached our maximum retry amount
    /// It will send a response otherwise
    async fn should_retry_processor_bridge_account_creation(
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
                Box::pin(self.should_retry_processor_bridge_account_creation(
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

    async fn should_retry_authorization_bridge_account_creation(
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
                Box::pin(self.should_retry_authorization_bridge_account_creation(
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

    pub async fn get_bridge_info(
        &self,
        main_chain: &str,
        sender_chain: &str,
        receive_chain: &str,
    ) -> Result<PolytoneSingleChainInfo, CosmosCosmwasmError> {
        let gc = GLOBAL_CONFIG.lock().await;

        let (sender_chain, other_chain) = if main_chain == sender_chain {
            (sender_chain, receive_chain)
        } else {
            (receive_chain, sender_chain)
        };

        gc.get_bridge_info(sender_chain, other_chain)?
            .get_polytone_info()
            .get(receive_chain)
            .context(format!("Bridge info not found for: {}", other_chain))
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

    pub async fn get_code_id_of_addr(&mut self, addr: String) -> Result<u64, CosmosCosmwasmError> {
        // find the code id of the address
        let code_id_req = QueryContractInfoRequest {
            address: addr.clone(),
        };

        // Query for the code id of the address
        Ok(self
            .wallet
            .client
            .clients
            .wasm
            .contract_info(code_id_req)
            .await
            .context(format!("Failed to query address info: {}", addr))
            .map_err(CosmosCosmwasmError::Error)?
            .into_inner()
            .contract_info
            .context(format!("No contract info found: {}", addr))
            .map_err(CosmosCosmwasmError::Error)?
            .code_id)
    }

    // We query the chain for the code id of the address, and look this code id into our list of code ids to get the contract name
    pub async fn get_contract_name_by_address(
        &mut self,
        addr: String,
    ) -> Result<String, CosmosCosmwasmError> {
        let code_id = self.get_code_id_of_addr(addr.clone()).await?;

        // find if code id is in our list of code ids and get the contract name
        Ok(self
            .code_ids
            .iter()
            .find(|(_, v)| **v == code_id)
            .context(format!("Code id not found: {} | {}", code_id, addr))
            .map_err(CosmosCosmwasmError::Error)?
            .0
            .clone())
    }
}
