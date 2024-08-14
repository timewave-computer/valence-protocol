use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::Config;

pub const PROCESSOR: Item<Addr> = Item::new("processor");
pub const CONFIG: Item<Config> = Item::new("config");
