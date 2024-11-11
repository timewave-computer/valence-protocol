use cw_storage_plus::Item;

pub const CONDITION: Item<bool> = Item::new("condition");
pub const EXECUTION_ID: Item<u64> = Item::new("execution_id");
