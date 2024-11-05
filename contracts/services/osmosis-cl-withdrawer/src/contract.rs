use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult,
    Uint64,
};

use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPLY_ID: u64 = 314;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(deps, env, info, msg, process_action, update_config)
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    new_config: ServiceConfigUpdate,
) -> Result<(), ServiceError> {
    new_config.update_config(deps)
}

pub fn process_action(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ActionMsgs,
    cfg: Config,
) -> Result<Response, ServiceError> {
    match msg {
        ActionMsgs::WithdrawLiquidity { position_id } => {
            try_liquidate_cl_position(deps, cfg, position_id)
        }
    }
}

pub fn try_liquidate_cl_position(
    deps: DepsMut,
    cfg: Config,
    position_id: Uint64,
) -> Result<Response, ServiceError> {
    // // we delegate the create position msg as a submsg as we will need to transfer
    // // the position afterwards. reply_always so that the saved state is cleared on error.
    // let delegated_input_acc_msgs = execute_submsgs_on_behalf_of(
    //     vec![SubMsg::reply_on_success(create_cl_position_msg, REPLY_ID)],
    //     Some(to_json_string(&cfg)?),
    //     &cfg.input_addr.clone(),
    // )?;

    // let service_submsg = SubMsg::reply_on_success(delegated_input_acc_msgs, REPLY_ID);

    Ok(Response::default())
    // .add_submessage(service_submsg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_service_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_service_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = valence_service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawServiceConfig {} => {
            let raw_config: ServiceConfig =
                valence_service_utils::raw_config::query_raw_service_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ServiceError> {
    match msg.id {
        // REPLY_ID => handle_liquidity_provision_reply_id(msg.result),
        _ => Err(ServiceError::Std(StdError::generic_err("unknown reply id"))),
    }
}
