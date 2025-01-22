use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;
use valence_middleware_utils::type_registry::types::ValenceType;

/// Approved libraries that can execute actions on behalf of the account
pub const APPROVED_LIBRARIES: Map<Addr, Empty> = Map::new("libraries");

pub const VALENCE_TYPE_STORE: Map<String, ValenceType> = Map::new("valence_type_store");
