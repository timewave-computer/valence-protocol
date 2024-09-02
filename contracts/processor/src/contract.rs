#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Reply, Response,
    StdResult, SubMsgResult,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner};
use valence_authorization_utils::{
    authorization::{ActionBatch, ExecutionType, Priority},
    callback::ExecutionResult,
    msg::ProcessorMessage,
};
use valence_processor_utils::{
    callback::PendingCallback,
    msg::{
        AuthorizationMsg, ExecuteMsg, InstantiateMsg, OwnerMsg, PermissionlessMsg,
        PolytoneContracts, QueryMsg,
    },
    processor::{Config, MessageBatch, Polytone, ProcessorDomain, State},
};

use crate::{
    callback::{
        create_callback_message, handle_successful_atomic_callback,
        handle_successful_non_atomic_callback, handle_unsuccessful_atomic_callback,
        handle_unsuccessful_non_atomic_callback,
    },
    error::{CallbackErrorReason, ContractError},
    queue::get_queue_map,
    state::{
        ATOMIC_BATCH_EXECUTION, CONFIG, EXECUTION_ID_TO_BATCH,
        NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX, PENDING_CALLBACK, RETRIES,
    },
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Set up owners and initial subowners
    initialize_owner(
        deps.storage,
        deps.api,
        Some(deps.api.addr_validate(&msg.owner)?.as_str()),
    )?;

    let config = Config {
        authorization_contract: deps.api.addr_validate(&msg.authorization_contract)?,
        processor_domain: match msg.polytone_contracts {
            Some(pc) => ProcessorDomain::External(Polytone {
                polytone_proxy_address: deps.api.addr_validate(&pc.polytone_proxy_address)?,
                polytone_note_address: deps.api.addr_validate(&pc.polytone_note_address)?,
            }),
            None => ProcessorDomain::Main,
        },
        state: State::Active,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "instantiate_processor"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => update_ownership(deps, env, info, action),
        ExecuteMsg::OwnerAction(owner_msg) => {
            assert_owner(deps.storage, &info.sender)?;
            match owner_msg {
                OwnerMsg::UpdateConfig {
                    authorization_contract,
                    polytone_contracts,
                } => update_config(deps, authorization_contract, polytone_contracts),
            }
        }
        ExecuteMsg::AuthorizationModuleAction(authorization_module_msg) => {
            let config = CONFIG.load(deps.storage)?;

            let authorized_sender = match config.processor_domain {
                ProcessorDomain::Main => config.authorization_contract,
                ProcessorDomain::External(polytone) => polytone.polytone_proxy_address,
            };

            if info.sender != authorized_sender {
                return Err(ContractError::Unauthorized {});
            }

            match authorization_module_msg {
                AuthorizationMsg::EnqueueMsgs {
                    id,
                    msgs,
                    action_batch,
                    priority,
                } => enqueue_messages(deps, id, msgs, action_batch, priority),
                AuthorizationMsg::RemoveMsgs {
                    queue_position,
                    priority,
                } => remove_messages(deps, queue_position, priority),
                AuthorizationMsg::AddMsgs {
                    id,
                    queue_position,
                    msgs,
                    action_batch,
                    priority,
                } => add_messages(deps, queue_position, id, msgs, action_batch, priority),
                AuthorizationMsg::Pause {} => pause_processor(deps),
                AuthorizationMsg::Resume {} => resume_processor(deps),
            }
        }
        ExecuteMsg::PermissionlessAction(permissionless_msg) => match permissionless_msg {
            PermissionlessMsg::Tick {} => process_tick(deps, env),
            PermissionlessMsg::Callback { execution_id, msg } => {
                process_callback(deps, env, info, execution_id, msg)
            }
        },
    }
}

fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::new().add_attributes(ownership.into_attributes()))
}

fn update_config(
    deps: DepsMut,
    authorization_contract: Option<String>,
    polytone_contracts: Option<PolytoneContracts>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(authorization_contract) = authorization_contract {
        config.authorization_contract = deps.api.addr_validate(&authorization_contract)?;
    }

    config.processor_domain = match polytone_contracts {
        Some(pc) => ProcessorDomain::External(Polytone {
            polytone_proxy_address: deps.api.addr_validate(&pc.polytone_proxy_address)?,
            polytone_note_address: deps.api.addr_validate(&pc.polytone_note_address)?,
        }),
        None => ProcessorDomain::Main,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "update_config"))
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
    action_batch: ActionBatch,
    priority: Priority,
) -> Result<Response, ContractError> {
    let queue = get_queue_map(&priority);

    let message_batch = MessageBatch {
        id,
        msgs,
        action_batch,
        priority,
    };
    queue.push_back(deps.storage, &message_batch)?;
    EXECUTION_ID_TO_BATCH.save(deps.storage, id, &message_batch)?;

    Ok(Response::new().add_attribute("method", "enqueue_messages"))
}

fn remove_messages(
    deps: DepsMut,
    queue_position: u64,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = get_queue_map(&priority);

    let batch = queue.remove_at(deps.storage, queue_position)?;
    let config = CONFIG.load(deps.storage)?;

    match batch {
        Some(batch) => {
            // Do the clean up and send the callback
            EXECUTION_ID_TO_BATCH.remove(deps.storage, batch.id);
            RETRIES.remove(deps.storage, batch.id);
            NON_ATOMIC_BATCH_CURRENT_ACTION_INDEX.remove(deps.storage, batch.id);
            PENDING_CALLBACK.remove(deps.storage, batch.id);
            let callback_msg =
                create_callback_message(&config, batch.id, ExecutionResult::RemovedByOwner)?;
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

/// Adds a set of messages in a specific position of the queue
fn add_messages(
    deps: DepsMut,
    queue_position: u64,
    id: u64,
    msgs: Vec<ProcessorMessage>,
    action_batch: ActionBatch,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = get_queue_map(&priority);

    let message_batch = MessageBatch {
        id,
        msgs,
        action_batch,
        priority,
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
            let current_retry = RETRIES.may_load(deps.storage, batch.id)?;
            // If the retry_cooldown has not passed yet, we'll push the batch back to the queue and wait for the next tick
            if let Some(current_retry) = current_retry {
                if !current_retry.retry_cooldown.is_expired(&env.block) {
                    queue.push_back(deps.storage, &batch)?;
                    return Ok(Response::new()
                        .add_attribute("method", "tick")
                        .add_attribute("action", "pushed_action_back_to_queue"));
                }
            }
            // First we check if the action batch is atomic or not, as the way of processing them is different
            match batch.action_batch.execution_type {
                ExecutionType::Atomic => {
                    // If the batch is atomic, we'll just try to execute all messages in the batch
                    messages = batch.create_atomic_messages();
                    // Add to atomic batch execution map so we know that it's being executed
                    ATOMIC_BATCH_EXECUTION.save(deps.storage, batch.id, &Empty {})?;
                }
                ExecutionType::NonAtomic => {
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
                    if let Some(callback) = batch.action_batch.actions[current_index]
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

fn process_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    execution_id: u64,
    msg: Binary,
) -> Result<Response, ContractError> {
    let pending_callback = PENDING_CALLBACK
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
    let config = CONFIG.load(deps.storage)?;
    let mut messages = vec![];
    // Check if the message sent is the one we are expecting
    // If it is, we'll proceed to next action or provide the callback to the authorization module (if we finished with all actions)
    // If it isn't, we need to see if we can retry the action or provide the error to the authorization module
    if msg != pending_callback.callback_msg {
        handle_unsuccessful_non_atomic_callback(
            deps.storage,
            index,
            execution_id,
            &pending_callback.message_batch,
            &mut messages,
            "Invalid callback message received".to_string(),
            &config,
            &env.block,
        )?;
    } else {
        handle_successful_non_atomic_callback(
            deps.storage,
            index,
            execution_id,
            &pending_callback.message_batch,
            &mut messages,
        )?;
    }

    // Remove the pending callback because we have processed it
    PENDING_CALLBACK.remove(deps.storage, execution_id);

    Ok(Response::new().add_attribute("method", "callback"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    // The reply logic will be different depending on the execution type of the batch
    // First we check if the reply comes from an atomic or non-atomic batch
    let config = CONFIG.load(deps.storage)?;
    let batch = EXECUTION_ID_TO_BATCH.load(deps.storage, msg.id)?;
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
                        )?;
                    }
                }
                SubMsgResult::Err(error) => {
                    handle_unsuccessful_non_atomic_callback(
                        deps.storage,
                        index,
                        msg.id,
                        &batch,
                        &mut messages,
                        error,
                        &config,
                        &env.block,
                    )?;
                }
            }
        }
        None => {
            // Atomic
            match msg.result {
                SubMsgResult::Ok(_) => {
                    // For atomic batches, we need to know that all the rest of the actions succeeded
                    // If it's not in the map, we've already requeued the batch and we don't need to do anything
                    if ATOMIC_BATCH_EXECUTION.has(deps.storage, msg.id) {
                        handle_successful_atomic_callback(&config, msg.id, &mut messages)?;
                    }
                }
                SubMsgResult::Err(error) => {
                    // If the action failed, we'll remove the batch from the atomic execution map and handle it
                    // If it's not here, we've already handled the unsuccessful callback and we don't need to do anything
                    if ATOMIC_BATCH_EXECUTION.has(deps.storage, msg.id) {
                        handle_unsuccessful_atomic_callback(
                            deps.storage,
                            msg.id,
                            &batch,
                            &mut messages,
                            error,
                            &config,
                            &env.block,
                        )?;
                        // Remove from the map as we've handled the unsuccessful callback
                        ATOMIC_BATCH_EXECUTION.remove(deps.storage, msg.id);
                    }
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
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::Config {} => to_json_binary(&get_config(deps)?),
        QueryMsg::GetQueue { from, to, priority } => {
            to_json_binary(&get_queue(deps, from, to, &priority)?)
        }
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
