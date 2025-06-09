// Purpose: State definitions for account factory contract
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

/// Tracks created accounts to prevent duplicates
pub const CREATED_ACCOUNTS: Map<Addr, bool> = Map::new("created_accounts");

/// Tracks used nonces to prevent replay attacks
pub const USED_NONCES: Map<(Addr, u64), bool> = Map::new("used_nonces");

/// Fee collector address
pub const FEE_COLLECTOR: Item<Option<Addr>> = Item::new("fee_collector");

/// JIT account contract code ID for Instantiate2
pub const JIT_ACCOUNT_CODE_ID: Item<u64> = Item::new("jit_account_code_id"); 