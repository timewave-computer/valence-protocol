use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const APPROVED_SERVICES: Map<Addr, bool> = Map::new("services");
