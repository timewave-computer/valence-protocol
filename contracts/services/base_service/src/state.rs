use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::ServiceConfig;

pub const PROCESSOR: Item<Addr> = Item::new("processor");
pub const CONFIG: Item<ServiceConfig> = Item::new("config");
