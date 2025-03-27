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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReserveId { addr } => execute::reserve_id(deps, addr),
        ExecuteMsg::SaveProgram {
            id,
            owner,
            program_config,
        } => execute::save_program(deps, &info, id, owner, program_config),
        ExecuteMsg::UpdateProgram { id, program_config } => {
            execute::update_program(deps, &info, id, program_config)
        }
    }
}

mod execute {
    use cosmwasm_std::{Binary, DepsMut, MessageInfo, Response};

    use crate::{
        state::{LAST_ID, PROGRAMS, PROGRAMS_BACKUP, PROGRAMS_OWNERS},
        ContractError,
    };

    pub fn reserve_id(deps: DepsMut, addr: String) -> Result<Response, ContractError> {
        let id = LAST_ID.load(deps.storage)? + 1;
        LAST_ID.save(deps.storage, &id)?;
        PROGRAMS_OWNERS.save(deps.storage, id, &deps.api.addr_validate(&addr)?)?;

        Ok(Response::new()
            .add_attribute("method", "reserve_id")
            .add_attribute("id", id.to_string()))
    }

    pub fn save_program(
        deps: DepsMut,
        info: &MessageInfo,
        id: u64,
        owner: String,
        program_config: Binary,
    ) -> Result<Response, ContractError> {
        // When id reserved, only that reserver can save program to this id
        let id_temp_owner = PROGRAMS_OWNERS.load(deps.storage, id)?;
        if id_temp_owner != info.sender {
            return Err(ContractError::UnauthorizedToSave(id_temp_owner.to_string()));
        }

        if PROGRAMS.has(deps.storage, id) {
            return Err(ContractError::ProgramAlreadyExists(id));
        } else {
            PROGRAMS.save(deps.storage, id, &program_config)?;
        }

        // After saving program, update owner so only the owner will be able to update this program
        PROGRAMS_OWNERS.save(deps.storage, id, &deps.api.addr_validate(&owner)?)?;

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
        let program_owner = PROGRAMS_OWNERS.load(deps.storage, id)?;
        if program_owner != info.sender {
            return Err(ContractError::UnauthorizedToUpdate(
                program_owner.to_string(),
            ));
        }

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
        QueryMsg::GetAllConfigs {
            start,
            end,
            limit,
            order,
        } => {
            let start = start.map(Bound::inclusive);
            let end = end.map(Bound::inclusive);
            let order = order.unwrap_or(cosmwasm_std::Order::Ascending);
            let limit = limit.unwrap_or(10) as usize;

            let configs = PROGRAMS
                .range(deps.storage, start, end, order)
                .take(limit)
                .map(|item| item.map(|(id, program_config)| ProgramResponse { id, program_config }))
                .collect::<Result<Vec<_>, _>>()?;

            to_json_binary(&configs)
        }
        QueryMsg::GetLastId {} => to_json_binary(&LAST_ID.load(deps.storage)?),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        from_json,
        testing::{message_info, mock_dependencies, mock_env},
    };
    use valence_program_registry_utils::ProgramResponse;

    #[test]
    fn test_all_configs_order() {
        let mut deps = mock_dependencies();
        let sender = deps.api.addr_make("creator");
        let info = message_info(&sender, &[]);

        // init contract
        super::instantiate(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            super::InstantiateMsg {
                admin: sender.to_string(),
            },
        )
        .unwrap();

        for i in 1..11 {
            // reserve id
            super::execute(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                super::ExecuteMsg::ReserveId {
                    addr: sender.to_string(),
                },
            )
            .unwrap();

            // save config
            super::execute(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                super::ExecuteMsg::SaveProgram {
                    id: i,
                    owner: sender.to_string(),
                    program_config: b"config1".to_vec().into(),
                },
            )
            .unwrap();
        }

        // query first 5 configs and assert Ascending order
        let res: Vec<ProgramResponse> = from_json(
            super::query(
                deps.as_ref(),
                mock_env(),
                super::QueryMsg::GetAllConfigs {
                    start: None,
                    end: None,
                    limit: Some(5),
                    order: Some(cosmwasm_std::Order::Ascending),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.len(), 5);
        assert_eq!(res[0].id, 1);
        assert_eq!(res[4].id, 5);

        // query last 5 configs and assert Descending order
        let res: Vec<ProgramResponse> = from_json(
            super::query(
                deps.as_ref(),
                mock_env(),
                super::QueryMsg::GetAllConfigs {
                    start: None,
                    end: None,
                    limit: Some(5),
                    order: Some(cosmwasm_std::Order::Descending),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.len(), 5);
        assert_eq!(res[0].id, 10);
        assert_eq!(res[4].id, 6);

        // test pagination
        let res: Vec<ProgramResponse> = from_json(
            super::query(
                deps.as_ref(),
                mock_env(),
                super::QueryMsg::GetAllConfigs {
                    start: None,
                    end: Some(8),
                    limit: Some(2),
                    order: Some(cosmwasm_std::Order::Descending),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.len(), 2);
        assert_eq!(res[0].id, 8);
        assert_eq!(res[1].id, 7);

        let res: Vec<ProgramResponse> = from_json(
            super::query(
                deps.as_ref(),
                mock_env(),
                super::QueryMsg::GetAllConfigs {
                    start: None,
                    end: Some(6),
                    limit: Some(2),
                    order: Some(cosmwasm_std::Order::Descending),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.len(), 2);
        assert_eq!(res[0].id, 6);
        assert_eq!(res[1].id, 5);
    }
}
