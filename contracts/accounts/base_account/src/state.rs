use cosmwasm_std::{Addr, Empty, Uint64};
use cw_storage_plus::{Deque, Map};

// Approved services that can execute actions on behalf of the account
pub const APPROVED_SERVICES: Map<Addr, Empty> = Map::new("services");

pub const REPLY_QUEUE: Deque<Uint64> = Deque::new("reply_queue");
