use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint128};
use cw_utils::Expiration;

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
pub struct Action {
    // Note: for V1, all actions will be executed in the same domain
    pub domain: Domain,
    pub message_info: MessageInfo,
    // We use String instead of Addr because it can be a contract address in other execution environments
    pub contract_address: String,
    // If no retry logic is provided, we will assume that the action can't be retried
    pub retry_logic: Option<RetryLogic>,
    // Only applicable for NonAtomic execution type batches. An action might need to receive a callback to be confirmed, in that case we will include the callback confirmation.
    // If not provided, we assume that correct execution of the message implies confirmation.
    pub callback_confirmation: Option<ActionCallback>,
}

#[cw_serde]
pub enum Domain {
    Main,
    External(String),
}

#[cw_serde]
pub struct MessageInfo {
    pub message_type: MessageType,
    pub message: Message,
}

#[cw_serde]
// Abstracting this because maybe we might have different message types in the future (e.g. Migration)
pub enum MessageType {
    ExecuteMsg,
}

#[cw_serde]
pub struct Message {
    // Name of the message that is passed to the contract, e.g. in CosmWasm: the snake_case name of the ExecuteMsg, how it's passed in the JSON
    pub name: String,
    pub params_restrictions: Option<ParamsRestrictions>,
}

#[cw_serde]
// ParamRestrictinos will be passed separating it by a "." character. Example: If we want to specify that the json must have a param "address" under "owner" included, we will use
// MustBeIncluded("owner.address")
pub enum ParamsRestrictions {
    MustBeIncluded(String),
    CannotBeIncluded(String),
    // Will check that the param defined in String is included, and if it is, we will compare it with Binary (after converting it to Binary)
    MustBeValue(String, Binary),
}

#[cw_serde]
pub struct RetryLogic {
    pub times: RetryTimes,
    pub interval: RetryInterval,
}

#[cw_serde]
pub enum RetryTimes {
    Indefinitely,
    Amount(u64),
}

#[cw_serde]
pub enum RetryInterval {
    Seconds(u64),
    Blocks(u64),
}

#[cw_serde]
pub struct ActionCallback {
    // ID of the action we are receiving a callback for
    pub action_id: Uint128,
    // Address of contract we should receive the Callback from
    pub contract_address: String,
    // What we should receive from the callback to consider the action completed
    pub callback_message: Binary,
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
