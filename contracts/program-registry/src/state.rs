use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};

pub const LAST_ID: Item<u64> = Item::new("id");
pub const PROGRAMS: Map<u64, Binary> = Map::new("programs");
pub const PROGRAMS_BACKUP: Map<u64, Binary> = Map::new("programs_backups");
