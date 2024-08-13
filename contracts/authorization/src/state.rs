use authorization_utils::authorization::Authorization;
use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

pub const SUB_OWNERS: Map<Addr, Empty> = Map::new("sub_owners");

pub const AUTHORIZATIONS: Map<String, Authorization> = Map::new("authorizations");
