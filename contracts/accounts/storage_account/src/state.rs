use cosmwasm_std::{Addr, Binary, Empty};
use cw_storage_plus::Map;

// Approved libraries that can execute actions on behalf of the account
pub const APPROVED_LIBRARIES: Map<Addr, Empty> = Map::new("libraries");

// string key indexed blob (`cosmwasm_std::Binary`) storage
pub const BLOB_STORE: Map<String, Binary> = Map::new("blob_store");
