#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, WasmMsg,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::COUNTER,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    COUNTER.save(deps.storage, &1)?;
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
        ExecuteMsg::WillSucceed {} => Ok(Response::new()),
        ExecuteMsg::WillSucceedEveryFiveTimes {} => {
            let mut counter = COUNTER.load(deps.storage)?;
            counter += 1;
            COUNTER.save(deps.storage, &counter)?;
            if counter % 5 == 0 {
                Ok(Response::new())
            } else {
                Err(ContractError::Std(StdError::generic_err(
                    "this is not a 5th call",
                )))
            }
        }
        ExecuteMsg::SendCallback { to, callback } => {
            let wasm_msg = WasmMsg::Execute {
                contract_addr: to,
                msg: callback,
                funds: vec![],
            };
            Ok(Response::new().add_message(wasm_msg))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Counter {} => to_json_binary(&COUNTER.load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    COUNTER.save(deps.storage, &msg.new_counter)?;
    Ok(Response::new())
}
