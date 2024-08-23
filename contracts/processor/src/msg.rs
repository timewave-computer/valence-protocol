use std::collections::VecDeque;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use valence_authorization_utils::authorization::{ActionBatch, Priority};
use valence_processor_utils::{
    processor::{Config, PolytoneContracts},
    queue::MessageBatch,
};

#[cw_serde]
pub struct InstantiateMsg {
    // If not provided, the owner will be the sender
    pub owner: Option<Addr>,
    pub authorization_contract: Addr,
    pub polytone_contracts: Option<PolytoneContracts>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    OwnerAction(OwnerMsg),
    AuthorizationModuleAction(AuthoriationMsg),
    PermissionlessAction(PermissionlessMsg),
}

#[cw_serde]
pub enum OwnerMsg {
    UpdateConfig { config: Config },
}

#[cw_serde]
pub enum AuthoriationMsg {
    EnqueueMsgs {
        // Used for the callback or to remove the messages
        id: u64,
        msgs: Vec<Binary>,
        action_batch: ActionBatch,
        priority: Priority,
    },
    RemoveMsgs {
        id: u64,
        priority: Priority,
    },
    AddMsgs {
        id: u64,
        queue_position: usize,
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
    #[returns(VecDeque<MessageBatch>)]
    GetQueue { priority: Priority },
}
