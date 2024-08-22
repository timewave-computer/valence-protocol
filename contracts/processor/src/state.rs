use std::collections::VecDeque;

use cw_storage_plus::Item;
use valence_processor_utils::{processor::Config, queue::MessageBatch};

pub const CONFIG: Item<Config> = Item::new("config");
pub const MED_PRIORITY_QUEUE: Item<VecDeque<MessageBatch>> = Item::new("med_priority_queue");
pub const HIGH_PRIORITY_QUEUE: Item<VecDeque<MessageBatch>> = Item::new("high_priority_queue");
