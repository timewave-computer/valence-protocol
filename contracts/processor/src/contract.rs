#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_json, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Reply,
    Response, StdResult, SubMsg, SubMsgResult, Uint64, WasmMsg,
};

use valence_authorization_utils::{
    authorization::{ActionsConfig, Priority},
    callback::ExecutionResult,
    domain::PolytoneProxyState,
    msg::ProcessorMessage,
};
use valence_polytone_utils::polytone::{
    Callback, CallbackMessage, CallbackRequest, PolytoneExecuteMsg,
};
use valence_processor_utils::{
    callback::{PendingCallback, PolytoneCallbackMsg, PolytoneCallbackState},
    msg::{
        AuthorizationMsg, ExecuteMsg, InstantiateMsg, InternalProcessorMsg, PermissionlessMsg,
        QueryMsg,
    },
    processor::{Config, MessageBatch, Polytone, ProcessorDomain, State},
};

use crate::{
    callback::{
        create_callback_message, handle_successful_atomic_callback,
        handle_successful_non_atomic_callback, handle_unsuccessful_callback,
    },
    error::{CallbackErrorReason, ContractError, UnauthorizedReason},
    queue::get_queue_map,
    state::{
        CONFIG, EXECUTION_ID_TO_BATCH, NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX, PENDING_CALLBACK,
        PENDING_POLYTONE_CALLBACKS,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        authorization_contract: deps.api.addr_validate(&msg.authorization_contract)?,
        processor_domain: match msg.polytone_contracts {
            Some(pc) => ProcessorDomain::External(Polytone {
                polytone_proxy_address: deps.api.addr_validate(&pc.polytone_proxy_address)?,
                polytone_note_address: deps.api.addr_validate(&pc.polytone_note_address)?,
                timeout_seconds: pc.timeout_seconds,
                proxy_on_main_domain_state: PolytoneProxyState::PendingResponse,
            }),
            None => ProcessorDomain::Main,
        },
        state: State::Active,
    };
    CONFIG.save(deps.storage, &config)?;

    // Create an empty array of messages to trigger the proxy creation if it's not the main domain's processor
    let response = match config.processor_domain {
        ProcessorDomain::Main => Response::default(),
        ProcessorDomain::External(polytone) => {
            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: polytone.polytone_note_address.to_string(),
                msg: to_json_binary(&PolytoneExecuteMsg::Execute {
                    msgs: vec![],
                    callback: Some(CallbackRequest {
                        receiver: env.contract.address.to_string(),
                        // Any string would work, we just need to know what we are getting the callback for
                        msg: to_json_binary(&PolytoneCallbackMsg::CreateProxy)?,
                    }),
                    timeout_seconds: Uint64::from(polytone.timeout_seconds),
                })?,
                funds: vec![],
            });
            Response::new().add_message(msg)
        }
    };

    Ok(response.add_attribute("method", "instantiate_processor"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AuthorizationModuleAction(authorization_module_msg) => {
            let config = CONFIG.load(deps.storage)?;

            let authorized_sender = match config.processor_domain {
                ProcessorDomain::Main => config.authorization_contract,
                ProcessorDomain::External(polytone) => polytone.polytone_proxy_address,
            };

            if info.sender != authorized_sender {
                return Err(ContractError::Unauthorized(
                    UnauthorizedReason::NotAuthorizationModule {},
                ));
            }

            match authorization_module_msg {
                AuthorizationMsg::EnqueueMsgs {
                    id,
                    msgs,
                    actions_config,
                    priority,
                } => enqueue_messages(deps, id, msgs, actions_config, priority),
                AuthorizationMsg::EvictMsgs {
                    queue_position,
                    priority,
                } => evict_messages(deps, env, queue_position, priority),
                AuthorizationMsg::InsertMsgs {
                    id,
                    queue_position,
                    msgs,
                    actions_config,
                    priority,
                } => insert_messages(deps, queue_position, id, msgs, actions_config, priority),
                AuthorizationMsg::Pause {} => pause_processor(deps),
                AuthorizationMsg::Resume {} => resume_processor(deps),
            }
        }
        ExecuteMsg::PermissionlessAction(permissionless_msg) => match permissionless_msg {
            PermissionlessMsg::Tick {} => process_tick(deps, env),
            PermissionlessMsg::RetryCallback { execution_id } => {
                retry_callback(deps, env, execution_id)
            }
            PermissionlessMsg::RetryBridgeCreation {} => retry_bridge_creation(deps, env),
        },
        ExecuteMsg::InternalProcessorAction(internal_processor_msg) => match internal_processor_msg
        {
            InternalProcessorMsg::Callback { execution_id, msg } => {
                process_callback(deps, env, info, execution_id, msg)
            }
            InternalProcessorMsg::ExecuteAtomic { batch } => execute_atomic(info, env, batch),
        },
        ExecuteMsg::PolytoneCallback(callback_msg) => {
            process_polytone_callback(deps, env, info, callback_msg)
        }
    }
}

/// Sets the processor to Paused state, no more messages will be processed until resumed
fn pause_processor(deps: DepsMut) -> Result<Response, ContractError> {
    CONFIG.update(deps.storage, |mut c| -> Result<_, ContractError> {
        c.state = State::Paused;
        Ok(c)
    })?;

    Ok(Response::new().add_attribute("method", "pause_processor"))
}

/// Activates the processor, if it was paused it will process messages again
fn resume_processor(deps: DepsMut) -> Result<Response, ContractError> {
    CONFIG.update(deps.storage, |mut c| -> Result<_, ContractError> {
        c.state = State::Active;
        Ok(c)
    })?;

    Ok(Response::new().add_attribute("method", "resume_processor"))
}

/// Adds the messages to the back of the corresponding queue
fn enqueue_messages(
    deps: DepsMut,
    id: u64,
    msgs: Vec<ProcessorMessage>,
    actions_config: ActionsConfig,
    priority: Priority,
) -> Result<Response, ContractError> {
    let queue = get_queue_map(&priority);

    let message_batch = MessageBatch {
        id,
        msgs,
        actions_config,
        priority,
        retry: None,
    };
    queue.push_back(deps.storage, &message_batch)?;
    EXECUTION_ID_TO_BATCH.save(deps.storage, id, &message_batch)?;

    Ok(Response::new().add_attribute("method", "enqueue_messages"))
}

fn evict_messages(
    deps: DepsMut,
    env: Env,
    queue_position: u64,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = get_queue_map(&priority);
    let batch = queue.remove_at(deps.storage, queue_position)?;

    match batch {
        Some(batch) => {
            let config = CONFIG.load(deps.storage)?;
            // Do the clean up and send the callback
            EXECUTION_ID_TO_BATCH.remove(deps.storage, batch.id);
            NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.remove(deps.storage, batch.id);
            PENDING_CALLBACK.remove(deps.storage, batch.id);
            let callback_msg = create_callback_message(
                deps.storage,
                &config,
                batch.id,
                ExecutionResult::RemovedByOwner,
                &env.contract.address,
            )?;
            Ok(Response::new()
                .add_message(callback_msg)
                .add_attribute("method", "remove_messages")
                .add_attribute("messages_removed", batch.msgs.len().to_string()))
        }
        // It doesn't even exist, we do nothing
        None => Ok(Response::new()
            .add_attribute("method", "remove_messages")
            .add_attribute("messages_removed", "0")),
    }
}

/// Insert a set of messages in a specific position of the queue
fn insert_messages(
    deps: DepsMut,
    queue_position: u64,
    id: u64,
    msgs: Vec<ProcessorMessage>,
    actions_config: ActionsConfig,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = get_queue_map(&priority);

    let message_batch = MessageBatch {
        id,
        msgs,
        actions_config,
        priority,
        retry: None,
    };

    queue.insert_at(deps.storage, queue_position, &message_batch)?;
    EXECUTION_ID_TO_BATCH.save(deps.storage, id, &message_batch)?;

    Ok(Response::new().add_attribute("method", "add_messages"))
}

fn process_tick(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // If the processor is paused we won't process any messages
    if config.state.eq(&State::Paused) {
        return Err(ContractError::ProcessorPaused {});
    }

    // If there is something in the high priority queue we'll process it first
    let mut queue = get_queue_map(&Priority::High);
    if queue.len(deps.storage)? == 0 {
        queue = get_queue_map(&Priority::Medium);
    }

    let message_batch = queue.pop_front(deps.storage)?;

    let messages;
    match message_batch {
        Some(batch) => {
            // First we check if the current batch or action to be executed is retriable, if it isn't we'll just push it back to the end of the queue
            // If the retry_cooldown has not passed yet, we'll push the batch back to the queue and wait for the next tick
            if let Some(current_retry) = batch.retry.clone() {
                if !current_retry.retry_cooldown.is_expired(&env.block) {
                    queue.push_back(deps.storage, &batch)?;
                    return Ok(Response::new()
                        .add_attribute("method", "tick")
                        .add_attribute("action", "pushed_action_back_to_queue"));
                }
            }
            // First we check if the action batch is atomic or not, as the way of processing them is different
            match batch.actions_config {
                ActionsConfig::Atomic(_) => {
                    let id = batch.id;
                    // We'll trigger the processor to execute the batch atomically by calling himself
                    // Otherwise we can't execute it atomically
                    messages = vec![SubMsg::reply_always(
                        WasmMsg::Execute {
                            contract_addr: env.contract.address.to_string(),
                            msg: to_json_binary(&ExecuteMsg::InternalProcessorAction(
                                InternalProcessorMsg::ExecuteAtomic { batch },
                            ))?,
                            funds: vec![],
                        },
                        id,
                    )]
                }
                ActionsConfig::NonAtomic(ref actions_config) => {
                    // If the batch is non-atomic, we have to execute the message we are currently on
                    // If we never executed this batch before, we'll start from the first action (default - 0)
                    let index_stored =
                        NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.may_load(deps.storage, batch.id)?;

                    // If it's the first execution we'll start from the first action
                    let current_index = match index_stored {
                        Some(index) => index,
                        None => {
                            NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.save(
                                deps.storage,
                                batch.id,
                                &0,
                            )?;
                            0
                        }
                    };

                    // If the action is confirmed by a callback, let's create it and append the execution_id to the message
                    // If not, we don't need to append anything and just send it like it is
                    if let Some(callback) = actions_config.actions[current_index]
                        .callback_confirmation
                        .clone()
                    {
                        messages = batch
                            .create_message_by_index_with_execution_id(current_index, batch.id)?;
                        PENDING_CALLBACK.save(
                            deps.storage,
                            batch.id,
                            &PendingCallback {
                                address: callback.contract_address,
                                callback_msg: callback.callback_message,
                                message_batch: batch,
                            },
                        )?;
                    } else {
                        messages = batch.create_message_by_index(current_index)
                    };
                }
            }

            Ok(Response::new()
                .add_submessages(messages)
                .add_attribute("method", "tick"))
        }
        // Both queues are empty, there is nothing to do
        None => Err(ContractError::NoMessagesToProcess {}),
    }
}

fn retry_callback(deps: DepsMut, env: Env, execution_id: u64) -> Result<Response, ContractError> {
    let pending_callback = PENDING_POLYTONE_CALLBACKS
        .load(deps.storage, execution_id)
        .map_err(|_| {
            ContractError::CallbackError(CallbackErrorReason::PolytonePendingCallbackNotFound {})
        })?;

    // If the callback is not timed out (still pending) we won't retry it
    if pending_callback.state.ne(&PolytoneCallbackState::TimedOut) {
        return Err(ContractError::CallbackError(
            CallbackErrorReason::PolytoneCallbackStillPending {},
        ));
    }

    let config = CONFIG.load(deps.storage)?;
    let callback_msg = create_callback_message(
        deps.storage,
        &config,
        execution_id,
        pending_callback.execution_result,
        &env.contract.address,
    )?;

    Ok(Response::new()
        .add_message(callback_msg)
        .add_attribute("method", "retry_callback"))
}

fn retry_bridge_creation(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let polytone = match &mut config.processor_domain {
        ProcessorDomain::External(polytone) => polytone,
        ProcessorDomain::Main => {
            return Err(ContractError::Unauthorized(
                UnauthorizedReason::NotExternalDomainProcessor {},
            ))
        }
    };

    // If the proxy is not in a timedout state we won't retry it
    if polytone
        .proxy_on_main_domain_state
        .ne(&PolytoneProxyState::TimedOut)
    {
        return Err(ContractError::CallbackError(
            CallbackErrorReason::PolytoneCallbackNotRetriable {},
        ));
    }

    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: polytone.polytone_note_address.to_string(),
        msg: to_json_binary(&PolytoneExecuteMsg::Execute {
            msgs: vec![],
            callback: Some(CallbackRequest {
                receiver: env.contract.address.to_string(),
                // Any string would work, we just need to know what we are getting the callback for
                msg: to_json_binary(&PolytoneCallbackMsg::CreateProxy)?,
            }),
            timeout_seconds: Uint64::from(polytone.timeout_seconds),
        })?,
        funds: vec![],
    });

    // Update the state of the bridge creation so that we can't trigger this multiple times
    polytone.proxy_on_main_domain_state = PolytoneProxyState::PendingResponse;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("method", "retry_bridge_creation"))
}

fn process_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    execution_id: u64,
    msg: Binary,
) -> Result<Response, ContractError> {
    let mut pending_callback = PENDING_CALLBACK
        .load(deps.storage, execution_id)
        .map_err(|_| {
            ContractError::CallbackError(CallbackErrorReason::PendingCallbackNotFound {})
        })?;

    // Only the specified address for this ID can send a callback
    if info.sender != pending_callback.address {
        return Err(ContractError::CallbackError(
            CallbackErrorReason::InvalidCallbackSender {},
        ));
    }

    // Get the current index we are at for this non atomic action
    let index = NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.load(deps.storage, execution_id)?;
    let mut messages = vec![];
    // Check if the message sent is the one we are expecting
    // If it is, we'll proceed to next action or provide the callback to the authorization module (if we finished with all actions)
    // If it isn't, we need to see if we can retry the action or provide the error to the authorization module
    if msg != pending_callback.callback_msg {
        let config = CONFIG.load(deps.storage)?;
        handle_unsuccessful_callback(
            deps.storage,
            execution_id,
            &mut pending_callback.message_batch,
            &mut messages,
            "Invalid callback message received".to_string(),
            &config,
            &env.block,
            Some(index),
            &env.contract.address,
        )?;
    } else {
        handle_successful_non_atomic_callback(
            deps.storage,
            index,
            execution_id,
            &pending_callback.message_batch,
            &mut messages,
            &env.contract.address,
        )?;
    }

    // Remove the pending callback because we have processed it
    PENDING_CALLBACK.remove(deps.storage, execution_id);

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "callback"))
}

fn execute_atomic(
    info: MessageInfo,
    env: Env,
    batch: MessageBatch,
) -> Result<Response, ContractError> {
    // Only the processor can execute this
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized(
            UnauthorizedReason::NotProcessor {},
        ));
    }
    let messages: Vec<CosmosMsg> = batch.into();

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "execute_atomic"))
}

fn process_polytone_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMessage,
) -> Result<Response, ContractError> {
    // Check if the callback is from the processor
    if callback_msg.initiator != env.contract.address {
        return Err(ContractError::Unauthorized(
            UnauthorizedReason::InvalidPolytoneCallbackInitiator {},
        ));
    }

    let mut config = CONFIG.load(deps.storage)?;
    let polytone = match &mut config.processor_domain {
        ProcessorDomain::External(polytone) => polytone,
        ProcessorDomain::Main => {
            return Err(ContractError::Unauthorized(
                UnauthorizedReason::NotExternalDomainProcessor {},
            ))
        }
    };

    // Check if the sender is the authorized polytone note address
    if info.sender != polytone.polytone_note_address {
        return Err(ContractError::CallbackError(
            CallbackErrorReason::UnauthorizedPolytoneCallbackSender {},
        ));
    }

    match from_json::<PolytoneCallbackMsg>(callback_msg.initiator_msg.clone()) {
        Ok(polytone_callback_msg) => match polytone_callback_msg {
            PolytoneCallbackMsg::ExecutionID(execution_id) => match callback_msg.result {
                Callback::Execute(result) => match result {
                    Ok(_) => {
                        PENDING_POLYTONE_CALLBACKS.remove(deps.storage, execution_id);
                    }
                    Err(error) => {
                        PENDING_POLYTONE_CALLBACKS.update(
                            deps.storage,
                            execution_id,
                            |callback_info| -> Result<_, ContractError> {
                                match callback_info {
                                    Some(mut info) => {
                                        if error == "timeout" {
                                            info.state = PolytoneCallbackState::TimedOut;
                                        } else {
                                            info.state =
                                                PolytoneCallbackState::UnexpectedError(error);
                                        }
                                        Ok(info)
                                    }
                                    None => Err(ContractError::CallbackError(
                                        CallbackErrorReason::PolytonePendingCallbackNotFound {},
                                    )),
                                }
                            },
                        )?;
                    }
                },
                _ => {
                    return Err(ContractError::CallbackError(
                        CallbackErrorReason::InvalidPolytoneCallback {},
                    ));
                }
            },
            PolytoneCallbackMsg::CreateProxy => match callback_msg.result {
                Callback::Execute(result) => {
                    if result == Err("timeout".to_string()) {
                        polytone.proxy_on_main_domain_state = PolytoneProxyState::TimedOut
                    } else {
                        polytone.proxy_on_main_domain_state = PolytoneProxyState::Created
                    }
                }
                _ => {
                    return Err(ContractError::CallbackError(
                        CallbackErrorReason::InvalidPolytoneCallback {},
                    ));
                }
            },
        },
        // We should never enter here
        Err(_) => {
            return Err(ContractError::CallbackError(
                CallbackErrorReason::InvalidPolytoneCallback {},
            ));
        }
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "process_polytone_callback"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    // The reply logic will be different depending on the execution type of the batch
    // First we check if the reply comes from an atomic or non-atomic batch
    let config = CONFIG.load(deps.storage)?;
    let mut batch = EXECUTION_ID_TO_BATCH.load(deps.storage, msg.id)?;
    let mut messages = vec![];

    match NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.may_load(deps.storage, msg.id)? {
        Some(index) => {
            // Non Atomic
            // Check if it replied because of error or success
            match msg.result {
                SubMsgResult::Ok(_) => {
                    // If the action is only successful on a callback, we won't do anything because we'll wait for the callback instead
                    if !PENDING_CALLBACK.has(deps.storage, msg.id) {
                        handle_successful_non_atomic_callback(
                            deps.storage,
                            index,
                            msg.id,
                            &batch,
                            &mut messages,
                            &env.contract.address,
                        )?;
                    }
                }
                SubMsgResult::Err(error) => {
                    handle_unsuccessful_callback(
                        deps.storage,
                        msg.id,
                        &mut batch,
                        &mut messages,
                        error,
                        &config,
                        &env.block,
                        Some(index),
                        &env.contract.address,
                    )?;
                }
            }
        }
        None => {
            // Atomic
            match msg.result {
                SubMsgResult::Ok(_) => {
                    handle_successful_atomic_callback(
                        deps.storage,
                        &config,
                        msg.id,
                        &mut messages,
                        &env.contract.address,
                    )?;
                }
                SubMsgResult::Err(error) => {
                    handle_unsuccessful_callback(
                        deps.storage,
                        msg.id,
                        &mut batch,
                        &mut messages,
                        error,
                        &config,
                        &env.block,
                        None,
                        &env.contract.address,
                    )?;
                }
            }
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("method", "reply"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&get_config(deps)?),
        QueryMsg::GetQueue { from, to, priority } => {
            to_json_binary(&get_queue(deps, from, to, &priority)?)
        }
        QueryMsg::IsQueueEmpty {} => to_json_binary(&is_queue_empty(deps)?),
    }
}

fn get_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn get_queue(
    deps: Deps,
    from: Option<u64>,
    to: Option<u64>,
    priority: &Priority,
) -> StdResult<Vec<MessageBatch>> {
    let queue = get_queue_map(priority);
    queue.query(deps.storage, from, to, Order::Ascending)
}

fn is_queue_empty(deps: Deps) -> StdResult<bool> {
    let queue_high = get_queue_map(&Priority::High);
    let queue_med = get_queue_map(&Priority::Medium);

    Ok(queue_high.is_empty(deps.storage)? && queue_med.is_empty(deps.storage)?)
}
