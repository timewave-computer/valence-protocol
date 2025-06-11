use std::collections::HashMap;

use cosmwasm_std::Decimal;
use cw_storage_plus::Map;

// maps denom to a map of receiver_addr -> decimal_share
pub const DENOM_SPLITS: Map<String, HashMap<String, Decimal>> = Map::new("denom_splits");
