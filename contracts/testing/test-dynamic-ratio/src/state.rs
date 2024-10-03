use cosmwasm_std::Decimal;
use cw_storage_plus::Item;
use std::collections::HashMap;

pub const DENOM_RATIOS: Item<HashMap<String, Decimal>> = Item::new("denom_ratios");
