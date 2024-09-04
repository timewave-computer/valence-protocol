use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Uint128};
use cw_utils::Expiration;

use crate::action::{Action, AtomicAction, NonAtomicAction, RetryLogic};

#[cw_serde]
// What an owner or subowner can pass to the contract to create an authorization
pub struct AuthorizationInfo {
    // Unique ID for the authorization, will be used as denom of the TokenFactory token if needed
    pub label: String,
    pub mode: AuthorizationMode,
    pub not_before: Expiration,
    pub duration: AuthorizationDuration,
    // Default will be 1, defines how many times a specific authorization can be executed concurrently
    pub max_concurrent_executions: Option<u64>,
    pub actions_config: ActionsConfig,
    // If not passed, we will set the priority to Medium
    pub priority: Option<Priority>,
}

#[cw_serde]
pub enum AuthorizationDuration {
    Forever,
    Seconds(u64),
    Blocks(u64),
}

#[cw_serde]
// What we will save in the state of the Authorization contract for each label
pub struct Authorization {
    pub label: String,
    pub mode: AuthorizationMode,
    pub not_before: Expiration,
    pub expiration: Expiration,
    pub max_concurrent_executions: u64,
    pub actions_config: ActionsConfig,
    pub priority: Priority,
    pub state: AuthorizationState,
}

impl AuthorizationInfo {
    pub fn into_authorization(self, block_info: &BlockInfo) -> Authorization {
        let expiration = match self.duration {
            AuthorizationDuration::Forever => Expiration::Never {},
            AuthorizationDuration::Seconds(seconds) => {
                Expiration::AtTime(block_info.time.plus_seconds(seconds))
            }
            AuthorizationDuration::Blocks(blocks) => {
                Expiration::AtHeight(block_info.height + blocks)
            }
        };
        Authorization {
            label: self.label,
            mode: self.mode,
            not_before: self.not_before,
            expiration,
            max_concurrent_executions: self.max_concurrent_executions.unwrap_or(1),
            actions_config: self.actions_config,
            priority: self.priority.unwrap_or_default(),
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
pub enum ActionsConfig {
    Atomic(AtomicActionsConfig),
    NonAtomic(NonAtomicActionsConfig),
}

impl ActionsConfig {
    pub fn get_contract_address_by_action_index(&self, index: usize) -> &str {
        self.get_action_by_index(index)
            .map(|action| action.get_contract_address())
            .unwrap_or_default()
    }

    fn get_action_by_index(&self, index: usize) -> Option<&dyn Action> {
        match self {
            ActionsConfig::Atomic(config) => config.actions.get(index).map(|a| a as &dyn Action),
            ActionsConfig::NonAtomic(config) => config.actions.get(index).map(|a| a as &dyn Action),
        }
    }
}

#[cw_serde]
pub struct AtomicActionsConfig {
    pub actions: Vec<AtomicAction>,
    // Used for Atomic batches, if we don't specify retry logic then the actions won't be retried.
    pub retry_logic: Option<RetryLogic>,
}

#[cw_serde]
pub struct NonAtomicActionsConfig {
    pub actions: Vec<NonAtomicAction>,
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
