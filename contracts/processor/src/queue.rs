use cosmwasm_std::{BlockInfo, Storage};
use valence_authorization_utils::{authorization::Priority, function::RetryLogic};
use valence_processor_utils::{
    processor::{CurrentRetry, MessageBatch},
    queue::QueueMap,
};

use crate::{
    error::ContractError,
    state::{EXECUTION_ID_TO_BATCH, HIGH_PRIORITY_QUEUE, MED_PRIORITY_QUEUE},
};

pub fn get_queue_map(priority: &Priority) -> QueueMap<MessageBatch> {
    match priority {
        Priority::High => HIGH_PRIORITY_QUEUE,
        Priority::Medium => MED_PRIORITY_QUEUE,
    }
}

pub fn put_back_into_queue(
    storage: &mut dyn Storage,
    batch: &mut MessageBatch,
    retry_amounts: u64,
    retry_logic: &RetryLogic,
    block: &BlockInfo,
) -> Result<(), ContractError> {
    // Increment the retry for the batch
    batch.retry = Some(CurrentRetry {
        retry_amounts: retry_amounts.checked_add(1).expect("Overflow"),
        retry_cooldown: retry_logic.interval.after(block),
    });
    // Re-add to queue and save the batch with new retries
    let queue = get_queue_map(&batch.priority);
    queue.push_back(storage, batch)?;
    EXECUTION_ID_TO_BATCH.save(storage, batch.id, batch)?;

    Ok(())
}
