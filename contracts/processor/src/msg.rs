use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use valence_authorization_utils::authorization::{ActionBatch, Priority};
use valence_processor_utils::processor::{Config, MessageBatch};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub authorization_contract: String,
    // In case the processor is sitting on a different domain
    pub polytone_contracts: Option<PolytoneContracts>,
}

#[cw_serde]
pub struct PolytoneContracts {
    pub polytone_proxy_address: String,
    pub polytone_note_address: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    OwnerAction(OwnerMsg),
    AuthorizationModuleAction(AuthorizationMsg),
    PermissionlessAction(PermissionlessMsg),
}

#[cw_serde]
pub enum OwnerMsg {
    UpdateConfig {
        authorization_contract: Option<String>,
        polytone_contracts: Option<PolytoneContracts>,
    },
}

#[cw_serde]
pub enum AuthorizationMsg {
    EnqueueMsgs {
        // Used for the callback or to remove the messages
        id: u64,
        msgs: Vec<Binary>,
        action_batch: ActionBatch,
        priority: Priority,
    },
    RemoveMsgs {
        queue_position: u64,
        priority: Priority,
    },
    AddMsgs {
        queue_position: u64,
        id: u64,
        msgs: Vec<Binary>,
        action_batch: ActionBatch,
        priority: Priority,
    },
    Pause {},
    Resume {},
}

#[cw_serde]
pub enum PermissionlessMsg {
    Tick {},
}

#[cw_ownable_query]
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
