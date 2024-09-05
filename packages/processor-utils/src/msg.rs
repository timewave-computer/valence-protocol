use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use valence_authorization_utils::{
    authorization::{ActionsConfig, Priority},
    msg::ProcessorMessage,
};

use crate::processor::{Config, MessageBatch};

#[cw_serde]
pub struct InstantiateMsg {
    pub authorization_contract: String,
    // In case the processor is sitting on a different domain
    pub polytone_contracts: Option<PolytoneContracts>,
}

#[cw_serde]
pub struct PolytoneContracts {
    pub polytone_proxy_address: String,
    pub polytone_note_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AuthorizationModuleAction(AuthorizationMsg),
    PermissionlessAction(PermissionlessMsg),
    InternalProcessorAction(InternalProcessorMsg),
}

#[cw_serde]
pub enum AuthorizationMsg {
    EnqueueMsgs {
        // Used for the callback or to remove the messages
        id: u64,
        msgs: Vec<ProcessorMessage>,
        actions_config: ActionsConfig,
        priority: Priority,
    },
    EvictMsgs {
        queue_position: u64,
        priority: Priority,
    },
    InsertMsgs {
        queue_position: u64,
        id: u64,
        msgs: Vec<ProcessorMessage>,
        actions_config: ActionsConfig,
        priority: Priority,
    },
    Pause {},
    Resume {},
}

#[cw_serde]
pub enum PermissionlessMsg {
    Tick {},
}

#[cw_serde]
pub enum InternalProcessorMsg {
    Callback { execution_id: u64, msg: Binary },
    // Entry point for the processor to execute batches atomically, this will only be able to be called by the processor itself
    ExecuteAtomic { batch: MessageBatch },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(Vec<MessageBatch>)]
    GetQueue {
        from: Option<u64>,
        to: Option<u64>,
        priority: Priority,
    },
}
