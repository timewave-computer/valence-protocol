use std::collections::VecDeque;

use cosmwasm_std::{StdResult, Storage};
use valence_authorization_utils::authorization::Priority;
use valence_processor_utils::queue::MessageBatch;

use crate::state::{HIGH_PRIORITY_QUEUE, MED_PRIORITY_QUEUE};

pub fn load_queue(store: &dyn Storage, priority: &Priority) -> StdResult<VecDeque<MessageBatch>> {
    match priority {
        Priority::Medium => MED_PRIORITY_QUEUE.load(store),
        Priority::High => HIGH_PRIORITY_QUEUE.load(store),
    }
}

pub fn save_queue(
    store: &mut dyn Storage,
    priority: &Priority,
    queue: &VecDeque<MessageBatch>,
) -> StdResult<()> {
    match priority {
        Priority::Medium => MED_PRIORITY_QUEUE.save(store, queue),
        Priority::High => HIGH_PRIORITY_QUEUE.save(store, queue),
    }
}
