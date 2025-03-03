use cw_storage_plus::{Item, Map};
use valence_ibc_utils::neutron::Transfer;

/// contains all transfers mapped by a recipient address observed by the contract.
pub const RECIPIENT_TXS: Map<String, Vec<Transfer>> = Map::new("recipient_txs");
/// contains number of transfers to addresses observed by the contract.
pub const TRANSFERS: Item<u64> = Item::new("transfers");

pub const CATCHALL: Map<String, String> = Map::new("catchall");
