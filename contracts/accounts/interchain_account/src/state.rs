use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

use crate::msg::{IcaState, RemoteChainInformation};

// Approved libraries that can execute actions on behalf of the account
pub const APPROVED_LIBRARIES: Map<Addr, Empty> = Map::new("libraries");
// Remote chain information
pub const REMOTE_CHAIN_INFO: Item<RemoteChainInformation> = Item::new("remote_chain_info");
// State of the ICA
pub const ICA_STATE: Item<IcaState> = Item::new("ica_state");
