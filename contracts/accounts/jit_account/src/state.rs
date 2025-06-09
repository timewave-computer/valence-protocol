// Purpose: State definitions for JIT account contract
use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

/// Controller address that can approve libraries and execute messages
pub const CONTROLLER: Item<Addr> = Item::new("controller");

/// Map of approved libraries that can execute messages on behalf of the account
pub const APPROVED_LIBRARIES: Map<Addr, Empty> = Map::new("approved_libraries");

/// Account type: 1=TokenCustody, 2=DataStorage, 3=Hybrid
pub const ACCOUNT_TYPE: Item<u8> = Item::new("account_type"); 