use cosmwasm_std::{to_json_binary, Addr, BlockInfo, CosmosMsg, Storage, Uint64, WasmMsg};
use valence_authorization_utils::{
    authorization::Subroutine,
    callback::ExecutionResult,
    function::RetryTimes,
    msg::{ExecuteMsg, InternalAuthorizationMsg},
};
use valence_gmp_utils::polytone::{CallbackRequest, PolytoneExecuteMsg};
use valence_processor_utils::{
    callback::{PendingPolytoneCallbackInfo, PolytoneCallbackMsg, PolytoneCallbackState},
    processor::{Config, MessageBatch, ProcessorDomain},
};

use crate::{
    error::ContractError,
    queue::{get_queue_map, put_back_into_queue},
    state::{
        CONFIG, EXECUTION_ID_TO_BATCH, NON_ATOMIC_BATCH_CURRENT_FUNCTION_INDEX,
        PENDING_POLYTONE_CALLBACKS,
    },
};

pub fn create_callback_message(
    storage: &mut dyn Storage,
    config: &Config,
    execution_id: u64,
    execution_result: ExecutionResult,
    processor_address: &Addr,
) -> Result<CosmosMsg, ContractError> {
    // Message that will be sent to authorization contract
    let authorization_callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.authorization_contract.to_string(),
        msg: to_json_binary(&ExecuteMsg::InternalAuthorizationAction(
            InternalAuthorizationMsg::ProcessorCallback {
                execution_id,
                execution_result: execution_result.clone(),
            },
        ))?,
        funds: vec![],
    });

    let message = match &config.processor_domain {
        ProcessorDomain::Main => authorization_callback,
        // If it has to go through polytone we'll create the polytone message
        ProcessorDomain::External(polytone) => {
            // We store the pending callback so that we can track what is pending and what is timedout
            PENDING_POLYTONE_CALLBACKS.save(
                storage,
                execution_id,
                &PendingPolytoneCallbackInfo {
                    execution_result,
                    state: PolytoneCallbackState::Pending,
                },
            )?;

            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: polytone.polytone_note_address.to_string(),
                msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                    msgs: vec![authorization_callback],
                    callback: Some(CallbackRequest {
                        receiver: processor_address.to_string(),
                        // We'll return the execution ID to know for what we are receiving de callback for
                        msg: to_json_binary(&PolytoneCallbackMsg::ExecutionID(execution_id))?,
                    }),
                    timeout_seconds: Uint64::from(polytone.timeout_seconds),
                })?,
                funds: vec![],
            })
        }
    };

    Ok(message)
}

pub fn handle_successful_non_atomic_callback(
    storage: &mut dyn Storage,
    index: usize,
    execution_id: u64,
    batch: &mut MessageBatch,
    messages: &mut Vec<CosmosMsg>,
    processor_address: &Addr,
) -> Result<(), ContractError> {
    // Advance to the next function if there is one and if not, provide the successfull callback to the authorization module
    let next_index = index.checked_add(1).expect("Overflow");
    if next_index >= batch.msgs.len() {
        // We finished the batch, we'll provide the successfull callback to the authorization module
        let config = CONFIG.load(storage)?;
        messages.push(create_callback_message(
            storage,
            &config,
            execution_id,
            ExecutionResult::Success,
            processor_address,
        )?);

        // Clean up
        NON_ATOMIC_BATCH_CURRENT_FUNCTION_INDEX.remove(storage, execution_id);
        EXECUTION_ID_TO_BATCH.remove(storage, execution_id);
    } else {
        // We have more functions to process
        // Increase the index, reset retries and re-add batch to the queue
        NON_ATOMIC_BATCH_CURRENT_FUNCTION_INDEX.save(storage, execution_id, &next_index)?;
        batch.retry = None;
        let queue = get_queue_map(&batch.priority);
        queue.push_back(storage, batch)?;
    }

    Ok(())
}

pub fn handle_successful_atomic_callback(
    storage: &mut dyn Storage,
    config: &Config,
    execution_id: u64,
    messages: &mut Vec<CosmosMsg>,
    processor_address: &Addr,
) -> Result<(), ContractError> {
    messages.push(create_callback_message(
        storage,
        config,
        execution_id,
        ExecutionResult::Success,
        processor_address,
    )?);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_unsuccessful_callback(
    storage: &mut dyn Storage,
    execution_id: u64,
    batch: &mut MessageBatch,
    messages: &mut Vec<CosmosMsg>,
    error: String,
    config: &Config,
    block: &BlockInfo,
    index: Option<usize>,
    processor_address: &Addr,
) -> Result<(), ContractError> {
    let retry_logic = match &batch.subroutine {
        Subroutine::Atomic(config) => config.retry_logic.clone(),
        Subroutine::NonAtomic(config) => {
            let index = index.unwrap_or_default();
            config
                .functions
                .get(index)
                .and_then(|function| function.retry_logic.clone())
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
                        // We've retried the function the maximum amount of times, we'll provide the error callback to the authorization module
                        let execution_result = if index == 0 {
                            ExecutionResult::Rejected(error)
                        } else {
                            ExecutionResult::PartiallyExecuted(index, error)
                        };

                        messages.push(create_callback_message(
                            storage,
                            config,
                            execution_id,
                            execution_result,
                            processor_address,
                        )?);
                        // Clean up
                        NON_ATOMIC_BATCH_CURRENT_FUNCTION_INDEX.remove(storage, execution_id);
                        EXECUTION_ID_TO_BATCH.remove(storage, execution_id);
                    } else {
                        put_back_into_queue(storage, batch, retry_amounts, &retry_logic, block)?;
                    }
                }
                RetryTimes::Indefinitely => {
                    // We'll retry the function indefinitely
                    put_back_into_queue(storage, batch, retry_amounts, &retry_logic, block)?;
                }
            }
        }
        None => {
            // No retry logic, return callback to authorization module
            messages.push(create_callback_message(
                storage,
                config,
                execution_id,
                ExecutionResult::Rejected(error),
                processor_address,
            )?);
            // Clean up for non-atomic case
            NON_ATOMIC_BATCH_CURRENT_FUNCTION_INDEX.remove(storage, execution_id);
            EXECUTION_ID_TO_BATCH.remove(storage, execution_id);
        }
    }

    Ok(())
}
