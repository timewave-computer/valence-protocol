use valence_processor_utils::queue::QueueMap;

use crate::msg::WithdrawalObligation;

pub const CLEARING_QUEUE: QueueMap<WithdrawalObligation> = QueueMap::new(
    "clearing_queue",
    "clearing_queue_start_index",
    "clearing_queue_end_index",
);
