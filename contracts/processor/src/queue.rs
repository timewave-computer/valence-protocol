use cosmwasm_std::{BlockInfo, Storage};
use valence_authorization_utils::{action::RetryLogic, authorization::Priority};
use valence_processor_utils::{
    processor::{CurrentRetry, MessageBatch},
    queue::QueueMap,
};

use crate::{
    error::ContractError,
    state::{HIGH_PRIORITY_QUEUE, MED_PRIORITY_QUEUE, RETRIES},
};

pub fn get_queue_map(priority: &Priority) -> QueueMap<MessageBatch> {
    match priority {
        Priority::High => HIGH_PRIORITY_QUEUE,
        Priority::Medium => MED_PRIORITY_QUEUE,
    }
}

pub fn put_back_into_queue(
    storage: &mut dyn Storage,
    execution_id: u64,
    batch: &MessageBatch,
    retry_amounts: u64,
    retry_logic: &RetryLogic,
    block: &BlockInfo,
) -> Result<(), ContractError> {
    RETRIES.save(
        storage,
        execution_id,
        &CurrentRetry {
            retry_amounts: retry_amounts.checked_add(1).expect("Overflow"),
            retry_cooldown: retry_logic.interval.after(block),
        },
    )?;
    // Re-add to queue
    let queue = get_queue_map(&batch.priority);
    queue.push_back(storage, batch)?;

    Ok(())
}
