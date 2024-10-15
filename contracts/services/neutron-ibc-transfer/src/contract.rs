#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::bindings::query::NeutronQuery;
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionsMsgs, Config, OptionalServiceConfig, QueryMsg, ServiceConfig};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionsMsgs, OptionalServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(
        deps,
        env,
        info,
        msg,
        actions::process_action,
        execute::update_config,
    )
}

mod actions {
    use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
    use neutron_sdk::bindings::query::NeutronQuery;
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::msg::{ActionsMsgs, Config};

    pub fn process_action(
        deps: DepsMut<NeutronQuery>,
        env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionsMsgs::IbcTransfer {} => {
                let balance = cfg.denom().query_balance(&deps.querier, cfg.input_addr())?;
                if balance < *cfg.amount() {
                    return Err(ServiceError::ExecutionError(format!(
                        "Insufficient balance for denom '{}' in config (required: {}, available: {}).",
                        cfg.denom(), cfg.amount(), balance,
                    )));
                }

                // IBC Transfer funds from input account to output account on the remote chain
                let block_time = env.block.time;
                let ibc_send_msg = valence_ibc_utils::neutron::ibc_send_message(
                    deps,
                    env,
                    cfg.remote_chain_info().channel_id.clone(),
                    cfg.remote_chain_info().port_id.clone(),
                    cfg.input_addr().to_string(),
                    cfg.output_addr().to_string(),
                    cfg.denom().to_string(),
                    cfg.amount().u128(),
                    cfg.memo().clone(),
                    None,
                    cfg.remote_chain_info()
                        .ibc_transfer_timeout
                        .map(|timeout| block_time.plus_seconds(timeout.u64()).nanos()),
                )
                .map_err(|err| ServiceError::ExecutionError(err.to_string()))?;

                let input_account_msgs =
                    execute_on_behalf_of(vec![ibc_send_msg], cfg.input_addr())?;

                Ok(Response::new()
                    .add_attribute("method", "ibc-transfer")
                    .add_message(input_account_msgs))
                // .add_messages(vec![ibc_send_msg]))
            }
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use neutron_sdk::bindings::query::NeutronQuery;
    use valence_service_utils::error::ServiceError;

    use crate::msg::{Config, OptionalServiceConfig};

    pub fn update_config(
        deps: &DepsMut<NeutronQuery>,
        _env: Env,
        _info: MessageInfo,
        config: &mut Config,
        new_config: OptionalServiceConfig,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps, config)
    }
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
    }
}
