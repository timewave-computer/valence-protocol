use authorization_utils::{authorization::Authorization, domain::ExternalDomain};
use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

pub const SUB_OWNERS: Map<Addr, Empty> = Map::new("sub_owners");
pub const AUTHORIZATIONS: Map<String, Authorization> = Map::new("authorizations");
pub const PROCESSOR_ON_MAIN_DOMAIN: Item<Addr> = Item::new("processor_on_main_domain");
pub const EXTERNAL_DOMAINS: Map<String, ExternalDomain> = Map::new("external_domains");
