use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Coin, Empty, Uint64};
use cw_storage_plus::Map;
use valence_processor_utils::queue::QueueMap;

/// set of registered obligation ids meant to prevent double filling
/// of any given obligation
pub const REGISTERED_OBLIGATION_IDS: Map<u64, Empty> = Map::new("reg_obl_id");

/// fifo queue storing the pending obligations
pub const CLEARING_QUEUE: QueueMap<WithdrawalObligation> = QueueMap::new(
    "clearing_queue",
    "clearing_queue_start_index",
    "clearing_queue_end_index",
);

/// unsettled liability sitting in the clearing queue
#[cw_serde]
pub struct WithdrawalObligation {
    /// where the payout is to be routed
    pub recipient: Addr,
    /// what is owed to the recipient
    pub payout_coins: Vec<Coin>,
    /// some unique identifier for the request
    pub id: Uint64,
    /// block when registration was enqueued
    pub enque_block: BlockInfo,
}
