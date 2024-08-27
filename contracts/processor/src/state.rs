use cw_storage_plus::Item;
use valence_processor_utils::{
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
