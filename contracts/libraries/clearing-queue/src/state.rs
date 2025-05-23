use cosmwasm_schema::cw_serde;
use cosmwasm_std::{BlockInfo, Coin, Uint256};
use valence_processor_utils::queue::QueueMap;

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
    pub recipient: String,
    /// what is owed to the recipient
    pub payout_coins: Vec<Coin>,
    /// some unique identifier for the request
    pub id: Uint256,
    /// block when registration was enqueued
    pub enque_block: BlockInfo,
}
