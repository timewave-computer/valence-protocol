use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const APPROVED_SERVICES: Map<Addr, Empty> = Map::new("services");
