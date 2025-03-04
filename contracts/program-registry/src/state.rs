use cosmwasm_std::{Addr, Binary};
use cw_storage_plus::{Item, Map};

pub const LAST_ID: Item<u64> = Item::new("id");
pub const PROGRAMS_OWNERS: Map<u64, Addr> = Map::new("programs_owners");
pub const PROGRAMS: Map<u64, Binary> = Map::new("programs");
pub const PROGRAMS_BACKUP: Map<u64, Binary> = Map::new("programs_backups");
