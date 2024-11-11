use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::msg::{DynamicRatioQueryMsg, DynamicRatioResponse};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg},
    state::DENOM_RATIOS,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    DENOM_RATIOS.save(deps.storage, &msg.denom_ratios)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("method", "execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: DynamicRatioQueryMsg) -> StdResult<Binary> {
    match msg {
        DynamicRatioQueryMsg::DynamicRatio { denoms, params: _ } => {
            let ratios = DENOM_RATIOS.load(deps.storage)?;
            let result: HashMap<_, _> = denoms
                .into_iter()
                .filter_map(|denom| ratios.get(&denom).map(|&ratio| (denom, ratio)))
                .collect();
            to_json_binary(&DynamicRatioResponse {
                denom_ratios: result,
            })
        }
    }
}
