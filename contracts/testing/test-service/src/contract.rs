#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, WasmMsg,
};
use valence_processor_utils::msg::{ExecuteMsg as ProcessorExecuteMsg, InternalProcessorMsg};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{CONDITION, EXECUTION_ID},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONDITION.save(deps.storage, &false)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::WillError { error } => Err(ContractError::Std(StdError::generic_err(error))),
        ExecuteMsg::WillSucceed { execution_id } => {
            if let Some(execution_id) = execution_id {
                EXECUTION_ID.save(deps.storage, &execution_id)?;
            }
            Ok(Response::new())
        }
        ExecuteMsg::WillSucceedIfTrue {} => {
            if CONDITION.load(deps.storage)? {
                Ok(Response::new())
            } else {
                Err(ContractError::Std(StdError::generic_err(
                    "Condition not met",
                )))
            }
        }
        ExecuteMsg::SendCallback { to, callback } => {
            let callback_msg = ProcessorExecuteMsg::InternalProcessorAction(
                InternalProcessorMsg::ServiceCallback {
                    execution_id: EXECUTION_ID.load(deps.storage)?,
                    msg: callback,
                },
            );

            let wasm_msg = WasmMsg::Execute {
                contract_addr: to,
                msg: to_json_binary(&callback_msg)?,
                funds: vec![],
            };
            Ok(Response::new().add_message(wasm_msg))
        }
        ExecuteMsg::SetCondition { condition } => {
            CONDITION.save(deps.storage, &condition)?;
            Ok(Response::new())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Condition {} => to_json_binary(&CONDITION.load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    match msg {
        MigrateMsg::Migrate { new_condition } => {
            CONDITION.save(deps.storage, &new_condition)?;
            Ok(Response::new())
        }
    }
}
