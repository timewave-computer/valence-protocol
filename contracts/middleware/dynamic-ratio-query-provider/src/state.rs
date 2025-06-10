use std::collections::HashMap;

use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{ Item, Map};

// maps denom to a map of receiver_addr -> decimal_share
pub const DENOM_SPLITS: Map<String, HashMap<String, Decimal>> = Map::new("denom_splits");

pub const ADMIN: Item<Addr> = Item::new("admin");
