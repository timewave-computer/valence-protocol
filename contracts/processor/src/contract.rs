#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner};
use valence_authorization_utils::authorization::{ActionBatch, Priority};
use valence_processor_utils::processor::{Config, MessageBatch, Polytone, ProcessorDomain, State};

use crate::{
    error::ContractError,
    msg::{
        AuthorizationMsg, ExecuteMsg, InstantiateMsg, OwnerMsg, PermissionlessMsg,
        PolytoneContracts, QueryMsg,
    },
    queue::get_queue_map,
    state::CONFIG,
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
            PermissionlessMsg::Tick {} => todo!(),
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
    msgs: Vec<Binary>,
    action_batch: ActionBatch,
    priority: Priority,
) -> Result<Response, ContractError> {
    let queue = get_queue_map(&priority);

    let message_batch = MessageBatch {
        id,
        msgs,
        action_batch,
    };
    queue.push_back(deps.storage, &message_batch)?;

    Ok(Response::new().add_attribute("method", "enqueue_messages"))
}

fn remove_messages(
    deps: DepsMut,
    queue_position: u64,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = get_queue_map(&priority);

    queue.remove_at(deps.storage, queue_position)?;

    Ok(Response::new().add_attribute("method", "remove_messages"))
}

/// Adds a set of messages in a specific position of the queue
fn add_messages(
    deps: DepsMut,
    queue_position: u64,
    id: u64,
    msgs: Vec<Binary>,
    action_batch: ActionBatch,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = get_queue_map(&priority);

    let message_batch = MessageBatch {
        id,
        msgs,
        action_batch,
    };

    queue.insert_at(deps.storage, queue_position, &message_batch)?;

    Ok(Response::new().add_attribute("method", "add_messages"))
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
