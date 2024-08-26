use cw_storage_plus::Item;
use valence_processor_utils::{
    processor::{Config, MessageBatch},
    queue::QueueMap,
};

pub const CONFIG: Item<Config> = Item::new("config");
pub const MED_PRIORITY_QUEUE: QueueMap<MessageBatch> = QueueMap::new("med_priority_queue");
pub const HIGH_PRIORITY_QUEUE: QueueMap<MessageBatch> = QueueMap::new("high_priority_queue");
