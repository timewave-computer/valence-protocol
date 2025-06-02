use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Coin, Uint64};
use cw_storage_plus::Map;
use valence_processor_utils::queue::QueueMap;

/// map of registered obligation id -> settlement status.
/// this map is also used for obligation registration checks to
/// ensure that any given obligation can be registered at most once.
pub const OBLIGATION_ID_TO_STATUS_MAP: Map<u64, ObligationStatus> = Map::new("reg_obl_id");

/// fifo queue storing the pending obligations
pub const CLEARING_QUEUE: QueueMap<WithdrawalObligation> = QueueMap::new(
    "clearing_queue",
    "clearing_queue_start_index",
    "clearing_queue_end_index",
);

/// obligation status enum
#[cw_serde]
pub enum ObligationStatus {
    InQueue,
    Processed,
    Error(String),
}

/// unsettled liability sitting in the clearing queue
#[cw_serde]
pub struct WithdrawalObligation {
    /// where the payout is to be routed
    pub recipient: Addr,
    /// what is owed to the recipient
    pub payout_coin: Coin,
    /// some unique identifier for the request
    pub id: Uint64,
    /// block when registration was enqueued
    pub enqueue_block: BlockInfo,
}
