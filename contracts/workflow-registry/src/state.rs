use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};

pub const LAST_ID: Item<u64> = Item::new("id");
pub const WORKFLOWS: Map<u64, Binary> = Map::new("workflows");
pub const WORKFLOWS_BACKUP: Map<u64, Binary> = Map::new("workflows_backups");
