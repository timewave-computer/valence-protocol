use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use valence_library_utils::msg::{DynamicRatioQueryMsg, DynamicRatioResponse};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg},
    state::DENOM_SPLITS,
};

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

    // save the specified denom splits. no validation here as it
    // is done by the splitter.
    for (denom, split) in msg.split_cfg.split_cfg.iter() {
        DENOM_SPLITS.save(deps.storage, denom.to_string(), split)?;
    }

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateRatios { split_cfg } => {
            ensure!(
                cw_ownable::is_owner(deps.storage, &info.sender)?,
                StdError::generic_err("unauthorized")
            );
            // save the specified denom splits. no validation here as it
            // is done on splitter level.
            for (denom, split) in split_cfg.split_cfg.iter() {
                DENOM_SPLITS.save(deps.storage, denom.to_string(), split)?;
            }
            Ok(Response::new().add_attribute("method", "execute"))
        }
        ExecuteMsg::UpdateOwnership(action) => update_ownership(deps, env, info, action),
    }
}

pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> StdResult<Response> {
    let result =
        cw_ownable::update_ownership(deps.into_empty(), &env.block, &info.sender, action.clone())
            .map_err(|e| StdError::generic_err(e.to_string()))?;
    Ok(Response::default()
        .add_attribute("method", "update_ownership")
        .add_attribute("action", format!("{action:?}"))
        .add_attribute("result", format!("{result:?}")))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: DynamicRatioQueryMsg) -> StdResult<Binary> {
    match msg {
        DynamicRatioQueryMsg::DynamicRatio { denoms, params } => {
            let receiver = deps.api.addr_validate(&params)?;
            let mut denom_ratios = HashMap::new();

            // we iterate over the requested denoms to fetch the ratios
            // for receiver
            for denom in denoms {
                // load the target denom
                let configured_splits = DENOM_SPLITS.load(deps.storage, denom.to_string())?;

                // get the receiver share from the denom map
                let receiver_share = match configured_splits.get(receiver.as_str()) {
                    Some(share) => share,
                    None => {
                        return Err(StdError::generic_err(format!(
                            "denom {denom} does not have an entry for receiver {receiver}"
                        )))
                    }
                };

                denom_ratios.insert(denom.to_string(), *receiver_share);
            }

            to_json_binary(&DynamicRatioResponse { denom_ratios })
        }
        DynamicRatioQueryMsg::Ownership {} => {
            to_json_binary(&cw_ownable::get_ownership(deps.storage)?)
        }
    }
}
