use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128, WasmMsg};
use cw_ownable::{cw_ownable_execute, cw_ownable_query, Expiration};

use crate::{
    authorization::{Authorization, AuthorizationInfo, Priority},
    authorization_message::MessageType,
    callback::ExecutionResult,
    domain::{Domain, ExternalDomain},
};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    // Sub-owners can be added later if needed
    pub sub_owners: Vec<String>,
    // Processor on Main domain
    pub processor: String,
    // External domains
    pub external_domains: Vec<ExternalDomain>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    OwnerAction(OwnerMsg),
    PermissionedAction(PermissionedMsg),
    PermissionlessAction(PermissionlessMsg),
}

#[cw_serde]
pub enum OwnerMsg {
    AddSubOwner { sub_owner: String },
    RemoveSubOwner { sub_owner: String },
}

#[cw_serde]
pub enum PermissionedMsg {
    AddExternalDomains {
        external_domains: Vec<ExternalDomain>,
    },
    CreateAuthorizations {
        authorizations: Vec<AuthorizationInfo>,
    },
    ModifyAuthorization {
        label: String,
        not_before: Option<Expiration>,
        expiration: Option<Expiration>,
        max_concurrent_executions: Option<u64>,
        priority: Option<Priority>,
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
    RemoveMsgs {
        // Which domain we are targetting
        domain: Domain,
        // position in the queue
        queue_position: u64,
        // what queue we are targetting
        priority: Priority,
    },
    // Method to add messages from an authorization to any queue
    AddMsgs {
        // The authorization label
        label: String,
        // Where and in which queue we are putting them
        queue_position: u64,
        priority: Priority,
        // Messages to add
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
}

#[cw_serde]
pub struct Mint {
    pub address: Addr,
    pub amount: Uint128,
}

#[cw_serde]
pub enum PermissionlessMsg {
    SendMsgs {
        label: String,
        messages: Vec<ProcessorMessage>,
    },
    Callback {
        execution_id: u64,
        execution_result: ExecutionResult,
    },
}

#[cw_serde]
pub enum ProcessorMessage {
    CosmwasmExecuteMsg { msg: Binary },
    CosmwasmMigrateMsg { code_id: u64, msg: Binary },
}

impl ProcessorMessage {
    pub fn get_message_type(&self) -> MessageType {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { .. } => MessageType::CosmwasmExecuteMsg,
            ProcessorMessage::CosmwasmMigrateMsg { .. } => MessageType::CosmwasmMigrateMsg,
        }
    }

    pub fn get_msg(&self) -> &Binary {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { msg } => msg,
            ProcessorMessage::CosmwasmMigrateMsg { msg, .. } => msg,
        }
    }

    pub fn to_wasm_message(&self, contract_addr: &str) -> WasmMsg {
        match self {
            ProcessorMessage::CosmwasmExecuteMsg { msg } => WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: msg.clone(),
                funds: vec![],
            },
            ProcessorMessage::CosmwasmMigrateMsg { code_id, msg } => WasmMsg::Migrate {
                contract_addr: contract_addr.to_string(),
                new_code_id: *code_id,
                msg: msg.clone(),
            },
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
    #[returns(Vec<Authorization>)]
    Authorizations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}
