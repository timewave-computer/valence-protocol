use cosmwasm_std::{to_json_binary, BlockInfo, Storage, WasmMsg};
use valence_authorization_utils::{
    action::RetryTimes,
    authorization::ActionsConfig,
    callback::ExecutionResult,
    msg::{ExecuteMsg, PermissionlessMsg},
};
use valence_processor_utils::processor::{Config, MessageBatch, ProcessorDomain};

use crate::{
    error::ContractError,
    queue::{get_queue_map, put_back_into_queue},
    state::{CONFIG, EXECUTION_ID_TO_BATCH, NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX},
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
        EXECUTION_ID_TO_BATCH.remove(storage, execution_id);
    } else {
        // We have more actions to process
        // Increase the index and re-add batch to the queue
        NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.save(storage, execution_id, &next_index)?;
        let queue = get_queue_map(&batch.priority);
        queue.push_back(storage, batch)?;
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

#[allow(clippy::too_many_arguments)]
pub fn handle_unsuccessful_callback(
    storage: &mut dyn Storage,
    execution_id: u64,
    batch: &mut MessageBatch,
    messages: &mut Vec<WasmMsg>,
    error: String,
    config: &Config,
    block: &BlockInfo,
    index: Option<usize>,
) -> Result<(), ContractError> {
    let retry_logic = match &batch.actions_config {
        ActionsConfig::Atomic(config) => config.retry_logic.clone(),
        ActionsConfig::NonAtomic(config) => {
            let index = index.unwrap_or_default();
            config
                .actions
                .get(index)
                .and_then(|action| action.retry_logic.clone())
        }
    };

    let retry_amounts = batch.retry.as_ref().map_or(0, |retry| retry.retry_amounts);
    // Will only be used for non-atomic batches
    let index = index.unwrap_or_default();

    match retry_logic {
        Some(retry_logic) => {
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
                        EXECUTION_ID_TO_BATCH.remove(storage, execution_id);
                    } else {
                        put_back_into_queue(storage, batch, retry_amounts, &retry_logic, block)?;
                    }
                }
                RetryTimes::Indefinitely => {
                    // We'll retry the action indefinitely
                    put_back_into_queue(storage, batch, retry_amounts, &retry_logic, block)?;
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
            // Clean up for non-atomic case
            NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.remove(storage, execution_id);
            EXECUTION_ID_TO_BATCH.remove(storage, execution_id);
        }
    }

    Ok(())
}
