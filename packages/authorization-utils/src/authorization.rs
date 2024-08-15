use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_utils::Expiration;

use crate::action::Action;

#[cw_serde]
// What an owner or subowner can pass to the contract to create an authorization
pub struct AuthorizationInfo {
    // Unique ID for the authorization, will be used as denom of the TokenFactory token if needed
    pub label: String,
    pub mode: AuthorizationMode,
    pub expiration: Expiration,
    // Default will be 1, defines how many times a specific authorization can be executed concurrently
    pub max_concurrent_executions: Option<u64>,
    pub action_batch: ActionBatch,
    // If not passed, we will assume that the authorization has medium priority
    pub priority: Option<Priority>,
}

#[cw_serde]
// What we will save in the state of the Authorization contract for each label
pub struct Authorization {
    pub label: String,
    pub mode: AuthorizationMode,
    pub expiration: Expiration,
    pub max_concurrent_executions: u64,
    pub action_batch: ActionBatch,
    pub priority: Priority,
    pub state: AuthorizationState,
}

impl From<AuthorizationInfo> for Authorization {
    fn from(info: AuthorizationInfo) -> Self {
        Authorization {
            label: info.label,
            mode: info.mode,
            expiration: info.expiration,
            max_concurrent_executions: info.max_concurrent_executions.unwrap_or(1),
            action_batch: info.action_batch,
            priority: info.priority.unwrap_or_default(),
            state: AuthorizationState::Enabled,
        }
    }
}

#[cw_serde]
pub enum AuthorizationMode {
    Permissioned(PermissionType),
    Permissionless,
}

#[cw_serde]
pub enum PermissionType {
    // With call limit, we will mint certain amount of tokens per address. Each time they execute successfully we'll burn the token they send
    WithCallLimit(Vec<(Addr, Uint128)>),
    // Without call limit we will mint 1 token per address and we will query the sender if he has the token to verify if he can execute the actions
    WithoutCallLimit(Vec<Addr>),
}

#[cw_serde]
pub struct ActionBatch {
    pub execution_type: ExecutionType,
    pub actions: Vec<Action>,
}

#[cw_serde]
pub enum ExecutionType {
    Atomic,
    NonAtomic,
}

#[cw_serde]
#[derive(Default)]
pub enum Priority {
    #[default]
    Medium,
    High,
}

#[cw_serde]
pub enum AuthorizationState {
    Enabled,
    Disabled,
}
