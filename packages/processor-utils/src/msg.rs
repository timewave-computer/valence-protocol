use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use valence_authorization_utils::{
    authorization::{Priority, Subroutine},
    msg::ProcessorMessage,
};
use valence_polytone_utils::polytone::CallbackMessage;

use crate::{
    callback::PendingPolytoneCallbackInfo,
    processor::{Config, MessageBatch},
};

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
    pub timeout_seconds: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    AuthorizationModuleAction(AuthorizationMsg),
    PermissionlessAction(PermissionlessMsg),
    InternalProcessorAction(InternalProcessorMsg),
    // Polytone callback listener
    #[serde(rename = "callback")]
    PolytoneCallback(CallbackMessage),
}

#[cw_serde]
pub enum AuthorizationMsg {
    EnqueueMsgs {
        // Used for the callback or to remove the messages
        id: u64,
        msgs: Vec<ProcessorMessage>,
        subroutine: Subroutine,
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
        subroutine: Subroutine,
        priority: Priority,
    },
    Pause {},
    Resume {},
}

#[cw_serde]
pub enum PermissionlessMsg {
    Tick {},
    RetryCallback { execution_id: u64 },
    RetryBridgeCreation {},
}

#[cw_serde]
pub enum InternalProcessorMsg {
    LibraryCallback { execution_id: u64, msg: Binary },
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
    #[returns(bool)]
    IsQueueEmpty {},
    #[returns(Vec<PendingPolytoneCallbackInfo>)]
    PendingPolytoneCallbacks {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(PendingPolytoneCallbackInfo)]
    PendingPolytoneCallback { execution_id: u64 },
}
