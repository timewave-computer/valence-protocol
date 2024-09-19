use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};
use valence_authorization_utils::{
    authorization::Authorization, callback::ProcessorCallbackInfo, domain::ExternalDomain,
};

pub const FIRST_OWNERSHIP: Item<bool> = Item::new("first_ownership");
pub const SUB_OWNERS: Map<Addr, Empty> = Map::new("sub_owners");
pub const AUTHORIZATIONS: Map<String, Authorization> = Map::new("authorizations");
pub const PROCESSOR_ON_MAIN_DOMAIN: Item<Addr> = Item::new("processor_on_main_domain");
pub const EXTERNAL_DOMAINS: Map<String, ExternalDomain> = Map::new("external_domains");
pub const EXECUTION_ID: Item<u64> = Item::new("execution_id");
// To track how many of each authorization are pending completion
pub const CURRENT_EXECUTIONS: Map<String, u64> = Map::new("current_executions");
// Track all the callbacks for the processor, if they haven't been processed yet they will be in ExecutionResult::InProcess
pub const PROCESSOR_CALLBACKS: Map<u64, ProcessorCallbackInfo> = Map::new("processor_callbacks");
