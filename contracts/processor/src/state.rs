use cw_storage_plus::{Item, Map};
use valence_processor_utils::{
    callback::PendingCallback,
    processor::{Config, CurrentRetry, MessageBatch},
    queue::QueueMap,
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const MED_PRIORITY_QUEUE: QueueMap<MessageBatch> = QueueMap::new(
    "med_priority_queue",
    "med_priority_queue_start_index",
    "med_priority_queue_end_index",
);
pub const HIGH_PRIORITY_QUEUE: QueueMap<MessageBatch> = QueueMap::new(
    "high_priority_queue",
    "high_priority_queue_start_index",
    "high_priority_queue_end_index",
);

// We need to track the current retry we are on for a specific batch. The Map key is the Batch Execution ID and the value is the CurrentRetry struct
pub const RETRIES: Map<u64, CurrentRetry> = Map::new("batch_retries");

// For Non atomic batches, we need to know what action we are currently on. The Map key is the Batch Execution ID and the value is the current action index
// This way we can know what RetryLogic to use since each action has a different one
pub const NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX: Map<u64, u64> =
    Map::new("non_atomic_batch_current_action");

// Here we keep all the Non atomic batches that are currently waiting for a callback to continue. When we receive a callback
// we will verify if the callback comes from the right address and that the message sent is the one we are expecting, if it is
// we will continue the batch execution, if it isn't it will be put back to the queue with the right retry logic
pub const PENDING_CALLBACK: Map<u64, PendingCallback> = Map::new("pending_callback");
