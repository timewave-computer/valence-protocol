#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult,
};
use neutron_sdk::{
    bindings::{msg::NeutronMsg, query::NeutronQuery},
    sudo::msg::SudoMsg,
};
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
) -> Result<Response<impl CustomMsg>, ServiceError> {
    valence_service_base::execute(
        deps,
        env,
        info,
        msg,
        actions::process_action,
        execute::update_config,
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    if valence_ibc_utils::neutron::is_ibc_transfer_reply(&msg) {
        return valence_ibc_utils::neutron::handle_ibc_transfer_reply::<Empty>(deps, env, msg);
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> StdResult<Response> {
    if valence_ibc_utils::neutron::is_ibc_transfer_sudo(&msg) {
        return valence_ibc_utils::neutron::handle_ibc_transfer_sudo(
            deps,
            env,
            msg,
            sudo::ibc_transfer_sudo_callback,
        );
    }
    Ok(Response::default())
}

mod actions {
    use cosmwasm_std::{
        to_json_binary, CosmosMsg, CustomMsg, DepsMut, Empty, Env, MessageInfo, Response, SubMsg,
        Uint128, WasmMsg,
    };
    use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::msg::{ActionsMsgs, Config};

    pub fn process_action(
        deps: DepsMut<NeutronQuery>,
        env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response<impl CustomMsg>, ServiceError> {
        match msg {
            ActionsMsgs::IbcTransfer {} => {
                let balance = cfg.denom().query_balance(&deps.querier, cfg.input_addr())?;
                if balance < *cfg.amount() {
                    return Err(ServiceError::ExecutionError(format!(
                        "Insufficient balance for denom '{}' in config (required: {}, available: {}).",
                        cfg.denom(), cfg.amount(), balance,
                    )));
                }

                // Send funds from input account to the IBC transfer service
                // let transfer_msg = cfg
                //     .denom()
                //     .get_transfer_to_message(&env.contract.address, *cfg.amount())?;

                // let local_send_msg = SubMsg::new(
                //     execute_on_behalf_of(vec![transfer_msg], cfg.input_addr())?
                //         .change_custom()
                //         .ok_or_else(|| {
                //             ServiceError::ExecutionError(
                //                 "Failed to change local send msg custom type.".to_owned(),
                //             )
                //         })?,
                // );

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
                    &Empty {},
                    None,
                    cfg.remote_chain_info()
                        .ibc_transfer_timeout
                        .map(|timeout| block_time.plus_seconds(timeout.u64()).nanos()),
                )
                .map_err(|err| ServiceError::ExecutionError(err.to_string()))?;
                // .change_custom::<Empty>()
                // .ok_or_else(|| {
                //     ServiceError::ExecutionError(
                //         "Failed to change transfer msg custom type.".to_owned(),
                //     )
                // })?;

                let input_account_msgs = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.input_addr().to_string(),
                    msg: to_json_binary(&valence_account_utils::msg::ExecuteMsg::ExecuteMsg {
                        msgs: vec![ibc_send_msg],
                    })?,
                    funds: vec![],
                });

                Ok(Response::<NeutronMsg>::new()
                    .add_attribute("method", "ibc-transfer")
                    .add_message(input_account_msgs))
                    // .add_messages(vec![ibc_send_msg]))
            }
            ActionsMsgs::RefundDust {} => {
                let balance = cfg
                    .denom()
                    .query_balance(&deps.querier, &env.contract.address)?;
                if balance == Uint128::zero() {
                    return Err(ServiceError::ExecutionError(format!(
                        "Zero balance for denom '{}': nothing to refund.",
                        cfg.denom(),
                    )));
                }

                // Send dust from IBC transfer service back to the input account
                let transfer_msg = cfg
                    .denom()
                    .get_transfer_to_message(cfg.input_addr(), balance)?
                    .change_custom::<NeutronMsg>()
                    .ok_or_else(|| {
                        ServiceError::ExecutionError(
                            "Failed to change transfer msg custom type.".to_owned(),
                        )
                    })?;

                Ok(Response::<NeutronMsg>::new()
                    .add_attribute("method", "refund-dust")
                    .add_message(transfer_msg))
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

mod sudo {
    use cosmwasm_std::{Deps, Empty, Response, StdResult};

    // Callback handler for Sudo payload
    // Different logic is possible depending on the type of the payload we saved in msg_with_sudo_callback() call
    // This allows us to distinguish different transfer message from each other.
    // For example some protocols can send one transfer to refund user for some action and another transfer to top up some balance.
    // Such different actions may require different handling of their responses.
    pub fn ibc_transfer_sudo_callback(deps: Deps, payload: Empty) -> StdResult<Response> {
        deps.api.debug(
            format!(
                "WASMDEBUG: ibc_transfer_sudo_callback: sudo payload: {:?}",
                payload
            )
            .as_str(),
        );
        Ok(Response::new())
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
