use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use valence_library_utils::msg::{DynamicRatioQueryMsg, DynamicRatioResponse};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg},
    state::{ADMIN, DENOM_SPLITS},
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admin_addr = deps.api.addr_validate(&msg.admin)?;

    ADMIN.save(deps.storage, &admin_addr)?;

    // save the specified denom splits. no validation here as it
    // is done by the splitter.
    for (denom, split) in msg.split_cfg.split_cfg.iter() {
        DENOM_SPLITS.save(deps.storage, denom.to_string(), &split)?;
    }

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateRatios { split_cfg } => {
            // save the specified denom splits. no validation here as it
            // is done by the splitter.
            for (denom, split) in split_cfg.split_cfg.iter() {
                DENOM_SPLITS.save(deps.storage, denom.to_string(), &split)?;
            }
            Ok(Response::new().add_attribute("method", "execute"))
        }
    }
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
    }
}
