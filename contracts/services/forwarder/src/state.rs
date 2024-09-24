use cosmwasm_std::BlockInfo;
use cw_storage_plus::Item;

pub const LAST_SUCCESSFUL_FORWARD: Item<BlockInfo> = Item::new("last_successful_forward");
