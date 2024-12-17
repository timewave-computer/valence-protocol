#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use valence_encoder_utils::msg::QueryMsg;

use crate::error::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, ContractError> {
    unimplemented!("This contract does not handle any messages, only queries")
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::IsValidEncodingInfo { library, function } => {
            to_json_binary(&is_valid_encoding_info(deps, library, function)?)
        }
        QueryMsg::Encode {
            library,
            function,
            msg,
        } => to_json_binary(&encode(deps, library, function, msg)?),
    }
}

fn is_valid_encoding_info(deps: Deps, library: String, function: String) -> StdResult<bool> {
    todo!()
}

fn encode(deps: Deps, library: String, function: String, msg: Binary) -> StdResult<Binary> {
    todo!()
}
