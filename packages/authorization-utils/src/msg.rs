use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Binary, StdError, StdResult, Uint128, WasmMsg};
use cw_ownable::{cw_ownable_execute, cw_ownable_query, Expiration};
use valence_gmp_utils::{
    hyperlane::{HandleMsg, InterchainSecurityModuleResponse, IsmSpecifierQueryMsg},
    polytone::CallbackMessage,
};

use crate::{
    authorization::{Authorization, AuthorizationInfo, Priority},
    authorization_message::MessageType,
    callback::{ExecutionResult, ProcessorCallbackInfo},
    domain::{
        CosmwasmBridge, Domain, Encoder, EvmBridge, ExecutionEnvironment, ExternalDomain,
        HyperlaneConnector, PolytoneConnectors, PolytoneNote, PolytoneProxyState,
    },
    zk_authorization::{ZkAuthorization, ZkAuthorizationInfo},
};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    // Sub-owners can be added later if needed
    pub sub_owners: Vec<String>,
    // Processor on Main domain
    pub processor: String,
}

#[cw_serde]
pub struct ExternalDomainInfo {
    pub name: String,
    pub execution_environment: ExecutionEnvironmentInfo,
    pub processor: String,
}

#[cw_serde]
pub enum ExecutionEnvironmentInfo {
    Cosmwasm(CosmwasmBridgeInfo),
    Evm(EncoderInfo, EvmBridgeInfo),
}

#[cw_serde]
pub enum CosmwasmBridgeInfo {
    Polytone(PolytoneConnectorsInfo),
}

#[cw_serde]
pub enum EvmBridgeInfo {
    Hyperlane(HyperlaneConnectorInfo),
}

#[cw_serde]
pub struct PolytoneConnectorsInfo {
    pub polytone_note: PolytoneNoteInfo,
    pub polytone_proxy: String,
}

#[cw_serde]
pub struct PolytoneNoteInfo {
    pub address: String,
    pub timeout_seconds: u64,
}

#[cw_serde]
pub struct EncoderInfo {
    pub broker_address: String,
    pub encoder_version: String,
}

#[cw_serde]
pub struct HyperlaneConnectorInfo {
    pub mailbox: String,
    pub domain_id: u32,
}

impl EncoderInfo {
    pub fn to_addr(&self, api: &dyn Api) -> StdResult<Addr> {
        api.addr_validate(&self.broker_address)
    }

    pub fn to_validated_encoder(&self, api: &dyn Api) -> StdResult<Encoder> {
        Ok(Encoder {
            broker_address: self.to_addr(api)?,
            encoder_version: self.encoder_version.clone(),
        })
    }
}

impl HyperlaneConnectorInfo {
    pub fn to_addr(&self, api: &dyn Api) -> StdResult<Addr> {
        api.addr_validate(&self.mailbox)
    }

    pub fn to_validated_hyperlane_connector(&self, api: &dyn Api) -> StdResult<HyperlaneConnector> {
        Ok(HyperlaneConnector {
            mailbox: self.to_addr(api)?,
            domain_id: self.domain_id,
        })
    }
}

impl PolytoneNoteInfo {
    pub fn to_addr(&self, api: &dyn Api) -> StdResult<Addr> {
        api.addr_validate(&self.address)
    }

    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
}

impl EvmBridgeInfo {
    pub fn to_validated_evm_bridge(self, api: &dyn Api) -> StdResult<EvmBridge> {
        match self {
            EvmBridgeInfo::Hyperlane(hyperlane_info) => Ok(EvmBridge::Hyperlane(
                hyperlane_info.to_validated_hyperlane_connector(api)?,
            )),
        }
    }
}

impl ExecutionEnvironmentInfo {
    pub fn into_execution_environment(self, api: &dyn Api) -> StdResult<ExecutionEnvironment> {
        match self {
            ExecutionEnvironmentInfo::Cosmwasm(bridge_info) => match bridge_info {
                CosmwasmBridgeInfo::Polytone(polytone_info) => Ok(ExecutionEnvironment::Cosmwasm(
                    CosmwasmBridge::Polytone(PolytoneConnectors {
                        polytone_note: PolytoneNote {
                            address: polytone_info.polytone_note.to_addr(api)?,
                            timeout_seconds: polytone_info.polytone_note.timeout_seconds,
                            state: PolytoneProxyState::PendingResponse,
                        },
                        polytone_proxy: api.addr_validate(&polytone_info.polytone_proxy)?,
                    }),
                )),
            },
            ExecutionEnvironmentInfo::Evm(encoder_info, bridge_info) => {
                Ok(ExecutionEnvironment::Evm(
                    encoder_info.to_validated_encoder(api)?,
                    bridge_info.to_validated_evm_bridge(api)?,
                ))
            }
        }
    }
}

impl ExternalDomainInfo {
    pub fn into_external_domain_validated(self, api: &dyn Api) -> StdResult<ExternalDomain> {
        Ok(ExternalDomain {
            name: self.name,
            execution_environment: self.execution_environment.into_execution_environment(api)?,
            processor: self.processor,
        })
    }
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    OwnerAction(OwnerMsg),
    PermissionedAction(PermissionedMsg),
    PermissionlessAction(PermissionlessMsg),
    InternalAuthorizationAction(InternalAuthorizationMsg),
    // Polytone callback listener
    #[serde(rename = "callback")]
    PolytoneCallback(CallbackMessage),
    // Hyperlane callback listener
    #[serde(rename = "handle")]
    HyperlaneCallback(HandleMsg),
}

#[cw_serde]
pub enum OwnerMsg {
    AddSubOwner { sub_owner: String },
    RemoveSubOwner { sub_owner: String },
}

#[cw_serde]
pub enum PermissionedMsg {
    AddExternalDomains {
        external_domains: Vec<ExternalDomainInfo>,
    },
    CreateAuthorizations {
        authorizations: Vec<AuthorizationInfo>,
    },
    CreateZkAuthorizations {
        zk_authorizations: Vec<ZkAuthorizationInfo>,
    },
    ModifyAuthorization {
        label: String,
        not_before: Option<Expiration>,
        expiration: Option<Expiration>,
        max_concurrent_executions: Option<u64>,
        priority: Option<Priority>,
    },
    ModifyZkAuthorization {
        label: String,
        validate_last_block_execution: Option<bool>,
    },
    DisableAuthorization {
        label: String,
    },
    EnableAuthorization {
        label: String,
    },
    // Mint authorizations is only used for permissioned authorizations
    MintAuthorizations {
        label: String,
        mints: Vec<Mint>,
    },
    // Method to remove any set of messages from any queue in any domain
    EvictMsgs {
        // Which domain we are targetting
        domain: Domain,
        // position in the queue
        queue_position: u64,
        // what queue we are targetting
        priority: Priority,
    },
    // Method to insert messages from an authorization to any queue
    InsertMsgs {
        // The authorization label
        label: String,
        // Where and in which queue we are putting them
        queue_position: u64,
        priority: Priority,
        // Messages to insert
        messages: Vec<ProcessorMessage>,
    },
    // Pause a processor in any domain
    PauseProcessor {
        domain: Domain,
    },
    // Resume a processor in any domain
    ResumeProcessor {
        domain: Domain,
    },
    // Set a verification gateway contract for ZK authorizations
    SetVerifierContract {
        tag: u64,
        contract: String,
    },
}

#[cw_serde]
pub struct Mint {
    pub address: String,
    pub amount: Uint128,
}

#[cw_serde]
pub enum PermissionlessMsg {
    SendMsgs {
        label: String,
        messages: Vec<ProcessorMessage>,
        // Used in case of timeouts for cross-domain transactions. If they fail due to a timeout for example, anyone can permissionlessly re-send them if they are not expired.
        // If no expiration is set, they won't be able to be re-sent.
        ttl: Option<Expiration>,
    },
    // Permissionless entry point that allows retrying messages that have timed out
    RetryMsgs {
        // The execution ID that the messages were sent with and timed out
        execution_id: u64,
    },
    RetryBridgeCreation {
        domain_name: String,
    },
    // Execute ZK authorization
    ExecuteZkAuthorization {
        label: String,
        message: Binary,
        proof: Binary,
        // Public inputs of domain proof
        domain_message: Binary,
        // Domain proof
        domain_proof: Binary,
    },
}

#[cw_serde]
pub enum InternalAuthorizationMsg {
    ProcessorCallback {
        execution_id: u64,
        execution_result: ExecutionResult,
    },
}

#[cw_serde]
pub enum ProcessorMessage {
    CosmwasmExecuteMsg { msg: Binary },
    CosmwasmMigrateMsg { code_id: u64, msg: Binary },
    EvmCall { msg: Binary },
    EvmRawCall { msg: Binary },
}

impl PartialEq<MessageType> for ProcessorMessage {
    fn eq(&self, other: &MessageType) -> bool {
        matches!(
            (self, other),
            (
                ProcessorMessage::CosmwasmExecuteMsg { .. },
                MessageType::CosmwasmExecuteMsg
            ) | (
                ProcessorMessage::CosmwasmMigrateMsg { .. },
                MessageType::CosmwasmMigrateMsg
            ) | (ProcessorMessage::EvmCall { .. }, MessageType::EvmCall(..))
                | (ProcessorMessage::EvmRawCall { .. }, MessageType::EvmRawCall)
        )
    }
}

impl ProcessorMessage {
    pub fn get_msg(&self) -> &Binary {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { msg } => msg,
            ProcessorMessage::CosmwasmMigrateMsg { msg, .. } => msg,
            ProcessorMessage::EvmCall { msg } => msg,
            ProcessorMessage::EvmRawCall { msg } => msg,
        }
    }

    pub fn set_msg(&mut self, msg: Binary) {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { msg: msg_ref } => *msg_ref = msg,
            ProcessorMessage::CosmwasmMigrateMsg { msg: msg_ref, .. } => *msg_ref = msg,
            ProcessorMessage::EvmCall { msg: msg_ref } => *msg_ref = msg,
            ProcessorMessage::EvmRawCall { msg: msg_ref } => *msg_ref = msg,
        }
    }

    pub fn to_wasm_message(&self, contract_addr: &str) -> Result<WasmMsg, StdError> {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { msg } => Ok(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: msg.clone(),
                funds: vec![],
            }),
            ProcessorMessage::CosmwasmMigrateMsg { code_id, msg } => Ok(WasmMsg::Migrate {
                contract_addr: contract_addr.to_string(),
                new_code_id: *code_id,
                msg: msg.clone(),
            }),
            ProcessorMessage::EvmCall { .. } | ProcessorMessage::EvmRawCall { .. } => {
                Err(StdError::generic_err("Msg type not supported"))
            }
        }
    }
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Addr>)]
    SubOwners {},
    #[returns(Addr)]
    Processor {},
    #[returns(Vec<ExternalDomain>)]
    ExternalDomains {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(ExternalDomain)]
    ExternalDomain { name: String },
    #[returns(Vec<Authorization>)]
    Authorizations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Vec<ZkAuthorization>)]
    ZkAuthorizations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Addr)]
    VerificationContract { tag: u64 },
    #[returns(Vec<ProcessorCallbackInfo>)]
    ProcessorCallbacks {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(ProcessorCallbackInfo)]
    ProcessorCallback { execution_id: u64 },

    // Entry point required to receive Hyperlane messages
    #[returns(InterchainSecurityModuleResponse)]
    IsmSpecifier(IsmSpecifierQueryMsg),
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Binary;

    #[test]
    fn test_cosmwasm_execute_msg_equality() {
        let msg = ProcessorMessage::CosmwasmExecuteMsg {
            msg: Binary::from(vec![1, 2, 3]),
        };
        let msg_type = MessageType::CosmwasmExecuteMsg;
        assert!(msg.eq(&msg_type));
    }

    #[test]
    fn test_cosmwasm_migrate_msg_equality() {
        let msg = ProcessorMessage::CosmwasmMigrateMsg {
            code_id: 1,
            msg: Binary::from(vec![4, 5, 6]),
        };
        let msg_type = MessageType::CosmwasmMigrateMsg;
        assert!(msg.eq(&msg_type));
    }

    #[test]
    fn test_evm_call_equality() {
        let bin_data = Binary::from(vec![7, 8, 9]);
        let msg = ProcessorMessage::EvmCall {
            msg: bin_data.clone(),
        };
        let msg_type = MessageType::EvmCall(
            EncoderInfo {
                broker_address: "any".to_string(),
                encoder_version: "any".to_string(),
            },
            "library".to_string(),
        );
        assert!(msg.eq(&msg_type));
    }

    #[test]
    fn test_evm_raw_call_equality() {
        let msg = ProcessorMessage::EvmRawCall {
            msg: Binary::from(vec![10, 11, 12]),
        };
        let msg_type = MessageType::EvmRawCall;
        assert!(msg.eq(&msg_type));
    }

    #[test]
    fn test_cosmwasm_execute_msg_inequality() {
        let msg = ProcessorMessage::CosmwasmExecuteMsg {
            msg: Binary::from(vec![1, 2, 3]),
        };
        let msg_type = MessageType::CosmwasmMigrateMsg;
        assert!(!msg.eq(&msg_type));
    }

    #[test]
    fn test_evm_call_inequality() {
        let msg = ProcessorMessage::EvmCall {
            msg: Binary::from(vec![7, 8, 9]),
        };
        let msg_type = MessageType::EvmRawCall;
        assert!(!msg.eq(&msg_type));
    }
}
