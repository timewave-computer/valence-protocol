// Purpose: State management for JIT account contract
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

/// Controller address (immutable after instantiation)
pub const CONTROLLER: Item<Addr> = Item::new("controller");

/// Approved libraries that can execute messages through this account
pub const APPROVED_LIBRARIES: Map<Addr, cosmwasm_std::Empty> = Map::new("approved_libraries");
