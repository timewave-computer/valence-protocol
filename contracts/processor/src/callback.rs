use cosmwasm_std::{to_json_binary, BlockInfo, Storage, WasmMsg};
use valence_authorization_utils::{
    action::RetryTimes,
    callback::ExecutionResult,
    msg::{ExecuteMsg, PermissionlessMsg},
};
use valence_processor_utils::processor::{Config, MessageBatch, ProcessorDomain};

use crate::{
    error::ContractError,
    queue::{get_queue_map, put_back_into_queue},
    state::{CONFIG, NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX, RETRIES},
};

pub fn create_callback_message(
    config: &Config,
    execution_id: u64,
    execution_result: ExecutionResult,
) -> Result<WasmMsg, ContractError> {
    let wasm_msg = match &config.processor_domain {
        ProcessorDomain::Main => WasmMsg::Execute {
            contract_addr: config.authorization_contract.to_string(),
            msg: to_json_binary(&ExecuteMsg::PermissionlessAction(
                PermissionlessMsg::Callback {
                    execution_id,
                    execution_result,
                },
            ))?,
            funds: vec![],
        },
        ProcessorDomain::External(_polytone) => todo!(),
    };
    Ok(wasm_msg)
}

pub fn handle_successful_non_atomic_callback(
    storage: &mut dyn Storage,
    index: usize,
    execution_id: u64,
    batch: &MessageBatch,
    messages: &mut Vec<WasmMsg>,
) -> Result<(), ContractError> {
    // Advance to the next action if there is one and if not, provide the successfull callback to the authorization module
    let next_index = index.checked_add(1).expect("Overflow");
    if next_index >= batch.msgs.len() {
        // We finished the batch, we'll provide the successfull callback to the authorization module
        let config = CONFIG.load(storage)?;
        messages.push(create_callback_message(
            &config,
            execution_id,
            ExecutionResult::Success,
        )?);

        // Clean up
        NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.remove(storage, execution_id);
    } else {
        // We have more actions to process
        // Increase the index and re-add batch to the queue
        NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.save(storage, execution_id, &next_index)?;
        let queue = get_queue_map(&batch.priority);
        queue.push_back(storage, batch)?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_unsuccessful_non_atomic_callback(
    storage: &mut dyn Storage,
    index: usize,
    execution_id: u64,
    batch: &MessageBatch,
    messages: &mut Vec<WasmMsg>,
    error: String,
    config: &Config,
    block: &BlockInfo,
) -> Result<(), ContractError> {
    // If the action failed, we'll retry it according to the retry policy or provide the error to the authorization module if
    // we reached the max amount of retries
    match &batch.action_batch.actions[index].retry_logic {
        Some(retry_logic) => {
            // Check how many retry amounts we have to keep track of them
            let retry_amounts = RETRIES
                .may_load(storage, execution_id)?
                .map_or(0, |r| r.retry_amounts);
            // Check if we reached the max amount of retries
            match &retry_logic.times {
                RetryTimes::Amount(max_retries) => {
                    if retry_amounts >= *max_retries {
                        // We've retried the action the maximum amount of times, we'll provide the error callback to the authorization module
                        let execution_result = if index == 0 {
                            ExecutionResult::Rejected(error)
                        } else {
                            ExecutionResult::PartiallyExecuted(index, error)
                        };

                        messages.push(create_callback_message(
                            config,
                            execution_id,
                            execution_result,
                        )?);
                        // Clean up
                        NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.remove(storage, execution_id);
                        RETRIES.remove(storage, execution_id);
                    }
                    // Otherwise, update values and re-add to queue
                    else {
                        put_back_into_queue(
                            storage,
                            execution_id,
                            batch,
                            retry_amounts,
                            retry_logic,
                            block,
                        )?;
                    }
                }
                RetryTimes::Indefinitely => {
                    // We'll retry the action indefinitely
                    put_back_into_queue(
                        storage,
                        execution_id,
                        batch,
                        retry_amounts,
                        retry_logic,
                        block,
                    )?;
                }
            }
        }
        None => {
            // No retry logic, return callback to authorization module
            messages.push(create_callback_message(
                config,
                execution_id,
                ExecutionResult::Rejected(error),
            )?);
            // Clean up
            NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.remove(storage, execution_id);
        }
    }

    Ok(())
}

pub fn handle_successful_atomic_callback(
    config: &Config,
    execution_id: u64,
    messages: &mut Vec<WasmMsg>,
) -> Result<(), ContractError> {
    messages.push(create_callback_message(
        config,
        execution_id,
        ExecutionResult::Success,
    )?);

    Ok(())
}

pub fn handle_unsuccessful_atomic_callback(
    storage: &mut dyn Storage,
    execution_id: u64,
    batch: &MessageBatch,
    messages: &mut Vec<WasmMsg>,
    error: String,
    config: &Config,
    block: &BlockInfo,
) -> Result<(), ContractError> {
    // If the action failed, we'll retry it according to the retry policy or provide the error to the authorization module
    let retry_amounts = RETRIES
        .may_load(storage, execution_id)?
        .map_or(0, |r| r.retry_amounts);

    match &batch.action_batch.retry_logic {
        Some(retry_logic) => {
            match &retry_logic.times {
                RetryTimes::Amount(max_retries) => {
                    if retry_amounts >= *max_retries {
                        // We've retried the action the maximum amount of times, we'll provide the error callback to the authorization module
                        messages.push(create_callback_message(
                            config,
                            execution_id,
                            ExecutionResult::Rejected(error),
                        )?);
                        // Clean up
                        RETRIES.remove(storage, execution_id);
                    } else {
                        put_back_into_queue(
                            storage,
                            execution_id,
                            batch,
                            retry_amounts,
                            retry_logic,
                            block,
                        )?;
                    }
                }
                RetryTimes::Indefinitely => {
                    // We'll retry the action indefinitely
                    put_back_into_queue(
                        storage,
                        execution_id,
                        batch,
                        retry_amounts,
                        retry_logic,
                        block,
                    )?;
                }
            }
        }
        None => {
            // No retries, return callback to authorization module
            messages.push(create_callback_message(
                config,
                execution_id,
                ExecutionResult::Rejected(error),
            )?);
        }
    }

    Ok(())
}
