use cw_storage_plus::{Item, Map};
use valence_processor_utils::{
    callback::{PendingCallback, PendingPolytoneCallbackInfo},
    processor::{Config, MessageBatch},
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
// We'll use this map to store the execution ID and the batch itself. This way we can retrieve the batch by ID to process retries
pub const EXECUTION_ID_TO_BATCH: Map<u64, MessageBatch> = Map::new("id_to_batch");

// For Non atomic batches, we need to know what function we are currently on. The Map key is the Batch Execution ID and the value is the current function index
// This way we can know what RetryLogic to use since each function has a different one
pub const NON_ATOMIC_BATCH_CURRENT_FUNCTION_INDEX: Map<u64, usize> =
    Map::new("non_atomic_batch_current_function_index");

// Here we keep all the Non atomic batches that are currently waiting for a callback to continue. When we receive a callback
// we will verify if the callback comes from the right address and that the message sent is the one we are expecting, if it is
// we will continue the batch execution, if it isn't it will be put back to the queue with the right retry logic
pub const PENDING_CALLBACK: Map<u64, PendingCallback> = Map::new("pending_callback");

// Pending and Timedout polytone callbacks will be stored here so that anyone can permissionlessly retry them if they are timedout
// The key will be the execution ID the callback was for and we will store the result and the status to re-send if the state is TimedOut
pub const PENDING_POLYTONE_CALLBACKS: Map<u64, PendingPolytoneCallbackInfo> =
    Map::new("pending_polytone_callbacks");
