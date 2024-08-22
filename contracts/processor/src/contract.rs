use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};
use cw_ownable::{assert_owner, initialize_owner};
use valence_processor_utils::processor::{Config, State};

use crate::{
    error::ContractError,
    msg::{AuthoriationMsg, ExecuteMsg, InstantiateMsg, OwnerMsg, PermissionlessMsg},
    state::CONFIG,
};

// pagination info for queries
const MAX_PAGE_LIMIT: u32 = 250;

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

            if info.sender != config.authorization_contract {
                return Err(ContractError::Unauthorized {});
            }
            match authorization_module_msg {
                AuthoriationMsg::EnqueueMsgs {
                    id,
                    msgs,
                    action_batch,
                    priority,
                } => todo!(),
                AuthoriationMsg::RemoveMsgs { id } => todo!(),
                AuthoriationMsg::AddMsgs {
                    id,
                    queue_position,
                    msgs,
                    action_batch,
                    priority,
                } => todo!(),
                AuthoriationMsg::Pause {} => todo!(),
                AuthoriationMsg::Resume {} => todo!(),
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
