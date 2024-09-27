use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::Map;

// Approved services that can execute actions on behalf of the account
pub const APPROVED_SERVICES: Map<Addr, Empty> = Map::new("services");
