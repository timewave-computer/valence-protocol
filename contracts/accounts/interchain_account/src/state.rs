use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};
use valence_account_utils::ica::{IcaState, RemoteDomainInfo};

// Approved libraries that can execute actions on behalf of the account
pub const APPROVED_LIBRARIES: Map<Addr, Empty> = Map::new("libraries");
// Remote domain information
pub const REMOTE_DOMAIN_INFO: Item<RemoteDomainInfo> = Item::new("remote_domain_info");
// State of the ICA
pub const ICA_STATE: Item<IcaState> = Item::new("ica_state");
