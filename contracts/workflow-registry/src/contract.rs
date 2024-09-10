#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::state::{WORKFLOWS, WORKFLOWS_BACKUP};
use crate::{error::ContractError, state::LAST_ID};
use valence_workflow_registry_utils::{ExecuteMsg, InstantiateMsg, QueryMsg, WorkflowResponse};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.admin))?;

    LAST_ID.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReserveId {} => execute::reserve_id(deps, &info),
        ExecuteMsg::SaveWorkflow {
            id,
            workflow_config,
        } => execute::save_workflow(deps, &info, id, workflow_config),
        ExecuteMsg::UpdateWorkflow {
            id,
            workflow_config,
        } => execute::update_workflow(deps, &info, id, workflow_config),
        ExecuteMsg::UpdateOwnership(ownership_action) => {
            cw_ownable::update_ownership(deps, &env.block, &info.sender, ownership_action)?;

            Ok(Response::new().add_attribute("method", "update_ownership"))
        }
    }
}

mod execute {
    use cosmwasm_std::{Binary, DepsMut, MessageInfo, Response};
    use cw_ownable::assert_owner;

    use crate::{
        state::{LAST_ID, WORKFLOWS, WORKFLOWS_BACKUP},
        ContractError,
    };

    pub fn reserve_id(deps: DepsMut, info: &MessageInfo) -> Result<Response, ContractError> {
        assert_owner(deps.storage, &info.sender)?;

        let id = LAST_ID.load(deps.storage)? + 1;
        LAST_ID.save(deps.storage, &id)?;

        Ok(Response::new()
            .add_attribute("method", "reserve_id")
            .add_attribute("id", id.to_string()))
    }

    pub fn save_workflow(
        deps: DepsMut,
        info: &MessageInfo,
        id: u64,
        workflow_config: Binary,
    ) -> Result<Response, ContractError> {
        assert_owner(deps.storage, &info.sender)?;

        if WORKFLOWS.has(deps.storage, id) {
            return Err(ContractError::WorkflowAlreadyExists(id));
        } else {
            WORKFLOWS.save(deps.storage, id, &workflow_config)?;
        }

        Ok(Response::new()
            .add_attribute("method", "get_id")
            .add_attribute("id", id.to_string()))
    }

    pub fn update_workflow(
        deps: DepsMut,
        info: &MessageInfo,
        id: u64,
        workflow_config: Binary,
    ) -> Result<Response, ContractError> {
        assert_owner(deps.storage, &info.sender)?;

        match WORKFLOWS.may_load(deps.storage, id)? {
            Some(previous_workflow) => {
                WORKFLOWS_BACKUP.save(deps.storage, id, &previous_workflow)?;
                WORKFLOWS.save(deps.storage, id, &workflow_config)?;
            }
            None => return Err(ContractError::WorkflowDoesntExists(id)),
        };

        Ok(Response::new()
            .add_attribute("method", "get_id")
            .add_attribute("id", id.to_string()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig { id } => {
            let config = WORKFLOWS.load(deps.storage, id)?;
            let workflow = WorkflowResponse {
                id,
                workflow_config: config,
            };
            to_json_binary(&workflow)
        }
        QueryMsg::GetConfigBackup { id } => {
            let config = WORKFLOWS_BACKUP.may_load(deps.storage, id)?;
            let workflow = config.map(|config| WorkflowResponse {
                id,
                workflow_config: config,
            });
            to_json_binary(&workflow)
        }
    }
}

#[cfg(test)]
mod tests {}
