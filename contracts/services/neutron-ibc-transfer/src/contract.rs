#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use neutron_sdk::bindings::query::NeutronQuery;
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionMsgs, Config, QueryMsg, ServiceConfig, ServiceConfigUpdate};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::instantiate(deps.into_empty(), CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionMsgs, ServiceConfigUpdate>,
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

    use crate::msg::{ActionMsgs, Config, IbcTransferAmount};

    pub fn process_action(
        deps: DepsMut<NeutronQuery>,
        env: Env,
        _info: MessageInfo,
        msg: ActionMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionMsgs::IbcTransfer {} => {
                let balance = cfg.denom().query_balance(&deps.querier, cfg.input_addr())?;

                let amount = match cfg.amount() {
                    IbcTransferAmount::FullAmount => balance,
                    IbcTransferAmount::FixedAmount(amount) => {
                        if balance < *amount {
                            return Err(ServiceError::ExecutionError(format!(
                                "Insufficient balance for denom '{}' in config (required: {}, available: {}).",
                                cfg.denom(), amount, balance,
                            )));
                        }
                        *amount
                    }
                };

                // IBC Transfer funds from input account to output account on the remote chain
                let block_time = env.block.time;
                let ibc_send_msg = valence_ibc_utils::neutron::ibc_send_message(
                    deps,
                    env,
                    cfg.remote_chain_info().channel_id.clone(),
                    cfg.input_addr().to_string(),
                    cfg.output_addr().to_string(),
                    cfg.denom().to_string(),
                    amount.u128(),
                    cfg.memo().clone(),
                    None,
                    cfg.remote_chain_info()
                        .ibc_transfer_timeout
                        .map(|timeout| block_time.plus_seconds(timeout.u64()).nanos()),
                    cfg.denom_to_pfm_map().clone(),
                )
                .map_err(|err| ServiceError::ExecutionError(err.to_string()))?;

                let input_account_msgs =
                    execute_on_behalf_of(vec![ibc_send_msg], cfg.input_addr())?;

                Ok(Response::new()
                    .add_attribute("method", "ibc-transfer")
                    .add_message(input_account_msgs))
            }
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use neutron_sdk::bindings::query::NeutronQuery;
    use valence_service_utils::error::ServiceError;

    use crate::msg::ServiceConfigUpdate;

    pub fn update_config(
        deps: DepsMut<NeutronQuery>,
        _env: Env,
        _info: MessageInfo,
        new_config: ServiceConfigUpdate,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
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
