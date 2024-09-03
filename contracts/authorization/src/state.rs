use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};
use valence_authorization_utils::{
    authorization::Authorization,
    callback::{CallbackInfo, PendingCallback},
    domain::ExternalDomain,
};

pub const SUB_OWNERS: Map<Addr, Empty> = Map::new("sub_owners");
pub const AUTHORIZATIONS: Map<String, Authorization> = Map::new("authorizations");
pub const PROCESSOR_ON_MAIN_DOMAIN: Item<Addr> = Item::new("processor_on_main_domain");
pub const EXTERNAL_DOMAINS: Map<String, ExternalDomain> = Map::new("external_domains");
pub const EXECUTION_ID: Item<u64> = Item::new("execution_id");
// To track how many of each authorization are pending completion
pub const CURRENT_EXECUTIONS: Map<String, u64> = Map::new("current_executions");
// Track the callbacks that are pending. Key is execution ID and value is the address that needs to send the callback (main domain's proccessor or callback proxy)
pub const PENDING_CALLBACKS: Map<u64, PendingCallback> = Map::new("pending_callback");
// Stores the confirmed callbacks for each execution ID for debugging purposes
pub const CONFIRMED_CALLBACKS: Map<u64, CallbackInfo> = Map::new("confirmed_callbacks");
