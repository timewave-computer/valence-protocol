use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw_ownable::{assert_owner, get_ownership, initialize_owner};
use valence_authorization_utils::authorization::{ActionBatch, Priority};
use valence_processor_utils::{
    processor::{Config, State},
    queue::MessageBatch,
};

use crate::{
    error::ContractError,
    msg::{AuthoriationMsg, ExecuteMsg, InstantiateMsg, OwnerMsg, PermissionlessMsg, QueryMsg},
    queue::{load_queue, save_queue},
    state::CONFIG,
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Set up owners and initial subowners
    initialize_owner(
        deps.storage,
        deps.api,
        Some(
            deps.api
                .addr_validate(msg.owner.unwrap_or(info.sender).as_str())?
                .as_str(),
        ),
    )?;

    let config = Config {
        authorization_contract: msg.authorization_contract,
        polytone_contracts: msg.polytone_contracts,
        state: State::Active,
    };
    config.is_valid(deps.api)?;

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
                OwnerMsg::UpdateConfig { config } => update_config(deps, config),
            }
        }
        ExecuteMsg::AuthorizationModuleAction(authorization_module_msg) => {
            let config = CONFIG.load(deps.storage)?;

            let authorized_sender = match config.polytone_contracts {
                Some(polytone_contracts) => polytone_contracts.polytone_proxy_contract,
                None => config.authorization_contract,
            };

            if info.sender != authorized_sender {
                return Err(ContractError::Unauthorized {});
            }

            match authorization_module_msg {
                AuthoriationMsg::EnqueueMsgs {
                    id,
                    msgs,
                    action_batch,
                    priority,
                } => enqueue_messages(deps, id, msgs, action_batch, priority),
                AuthoriationMsg::RemoveMsgs { id, priority } => remove_messages(deps, id, priority),
                AuthoriationMsg::AddMsgs {
                    id,
                    queue_position,
                    msgs,
                    action_batch,
                    priority,
                } => add_messages(deps, id, queue_position, msgs, action_batch, priority),
                AuthoriationMsg::Pause {} => pause_processor(deps),
                AuthoriationMsg::Resume {} => resume_processor(deps),
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

fn update_config(deps: DepsMut, config: Config) -> Result<Response, ContractError> {
    config.is_valid(deps.api)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "update_config"))
}

fn pause_processor(deps: DepsMut) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.state = State::Paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "pause_processor"))
}

fn resume_processor(deps: DepsMut) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.state = State::Active;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "resume_processor"))
}

fn enqueue_messages(
    deps: DepsMut,
    id: u64,
    msgs: Vec<Binary>,
    action_batch: ActionBatch,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = load_queue(deps.storage, &priority)?;

    let message_batch = MessageBatch {
        id,
        msgs,
        action_batch,
    };
    queue.push_back(message_batch);

    save_queue(deps.storage, &priority, &queue)?;

    Ok(Response::new().add_attribute("method", "enqueue_messages"))
}

fn remove_messages(deps: DepsMut, id: u64, priority: Priority) -> Result<Response, ContractError> {
    let mut queue = load_queue(deps.storage, &priority)?;

    queue.retain(|message_batch| message_batch.id != id);

    save_queue(deps.storage, &priority, &queue)?;

    Ok(Response::new().add_attribute("method", "remove_messages"))
}

fn add_messages(
    deps: DepsMut,
    id: u64,
    queue_position: usize,
    msgs: Vec<Binary>,
    action_batch: ActionBatch,
    priority: Priority,
) -> Result<Response, ContractError> {
    let mut queue = load_queue(deps.storage, &priority)?;

    let message_batch = MessageBatch {
        id,
        msgs,
        action_batch,
    };

    if queue_position > queue.len() {
        return Err(ContractError::InvalidQueuePosition {});
    }
    queue.insert(queue_position, message_batch);

    save_queue(deps.storage, &priority, &queue)?;

    Ok(Response::new().add_attribute("method", "add_messages"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
        QueryMsg::Config {} => to_json_binary(&get_config(deps)?),
        QueryMsg::GetQueue { priority } => to_json_binary(&load_queue(deps.storage, &priority)?),
    }
}

fn get_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
