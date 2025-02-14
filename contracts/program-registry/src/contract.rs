#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::state::{PROGRAMS, PROGRAMS_BACKUP};
use crate::{error::ContractError, state::LAST_ID};
use valence_program_registry_utils::{ExecuteMsg, InstantiateMsg, ProgramResponse, QueryMsg};

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
        ExecuteMsg::SaveProgram { id, program_config } => {
            execute::save_program(deps, &info, id, program_config)
        }
        ExecuteMsg::UpdateProgram { id, program_config } => {
            execute::update_program(deps, &info, id, program_config)
        }
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
        state::{LAST_ID, PROGRAMS, PROGRAMS_BACKUP},
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

    pub fn save_program(
        deps: DepsMut,
        info: &MessageInfo,
        id: u64,
        program_config: Binary,
    ) -> Result<Response, ContractError> {
        assert_owner(deps.storage, &info.sender)?;

        if PROGRAMS.has(deps.storage, id) {
            return Err(ContractError::ProgramAlreadyExists(id));
        } else {
            PROGRAMS.save(deps.storage, id, &program_config)?;
        }

        Ok(Response::new()
            .add_attribute("method", "get_id")
            .add_attribute("id", id.to_string()))
    }

    pub fn update_program(
        deps: DepsMut,
        info: &MessageInfo,
        id: u64,
        program_config: Binary,
    ) -> Result<Response, ContractError> {
        assert_owner(deps.storage, &info.sender)?;

        match PROGRAMS.may_load(deps.storage, id)? {
            Some(previous_program) => {
                PROGRAMS_BACKUP.save(deps.storage, id, &previous_program)?;
                PROGRAMS.save(deps.storage, id, &program_config)?;
            }
            None => return Err(ContractError::ProgramDoesntExists(id)),
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
            let config = PROGRAMS.load(deps.storage, id)?;
            let program = ProgramResponse {
                id,
                program_config: config,
            };
            to_json_binary(&program)
        }
        QueryMsg::GetConfigBackup { id } => {
            let config = PROGRAMS_BACKUP.may_load(deps.storage, id)?;
            let program = config.map(|config| ProgramResponse {
                id,
                program_config: config,
            });
            to_json_binary(&program)
        }
        QueryMsg::GetAllConfigs { start, end, limit } => {
            let start = start.map(Bound::inclusive);
            let end = end.map(Bound::exclusive);
            let limit = limit.unwrap_or(10) as usize;
            let configs = PROGRAMS
                .range(deps.storage, start, end, cosmwasm_std::Order::Ascending)
                .take(limit)
                .collect::<Result<Vec<_>, _>>()?;

            to_json_binary(&configs)
        }
        QueryMsg::GetLastId {} => to_json_binary(&LAST_ID.load(deps.storage)?),
    }
}
