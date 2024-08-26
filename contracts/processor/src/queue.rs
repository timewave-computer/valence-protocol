use valence_authorization_utils::authorization::Priority;
use valence_processor_utils::{processor::MessageBatch, queue::QueueMap};

use crate::state::{HIGH_PRIORITY_QUEUE, MED_PRIORITY_QUEUE};

pub fn get_queue_map(priority: &Priority) -> QueueMap<MessageBatch> {
    match priority {
        Priority::High => HIGH_PRIORITY_QUEUE,
        Priority::Medium => MED_PRIORITY_QUEUE,
    }
}
