use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

// Approved libraries that can execute actions on behalf of the account
pub const APPROVED_LIBRARIES: Map<Addr, Empty> = Map::new("libraries");
