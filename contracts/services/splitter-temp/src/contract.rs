#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
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
    deps: DepsMut,
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
    use cosmwasm_std::{
        to_json_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
    };
    use valence_service_utils::error::ServiceError;

    use crate::msg::{ActionsMsgs, Config};

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionsMsgs::Split {} => {
                let mut messages: Vec<CosmosMsg> = vec![];

                cfg.splits.iter().try_for_each(|(denom, split)| {
                    // Query bank balance
                    let balance = deps.querier.query_balance(&cfg.input_addr, denom)?;

                    // TODO: Check that balance is not zero
                    if !balance.amount.is_zero() {
                        // TODO: change split to be percentage and not amounts
                        messages.extend(
                            split
                                .iter()
                                .map(|(addr, amount)| {
                                    let bank_msg = BankMsg::Send {
                                        to_address: addr.to_string()?,
                                        amount: vec![Coin {
                                            denom: denom.clone(),
                                            amount: *amount,
                                        }],
                                    };

                                    Ok(WasmMsg::Execute {
                                        contract_addr: cfg.input_addr.to_string(),
                                        msg: to_json_binary(
                                            &valence_base_account::msg::ExecuteMsg::ExecuteMsg {
                                                msgs: vec![bank_msg.into()],
                                            },
                                        )?,
                                        funds: vec![],
                                    }
                                    .into())
                                })
                                .collect::<Result<Vec<_>, ServiceError>>()?,
                        );
                    }

                    Ok::<(), ServiceError>(())
                })?;

                Ok(Response::new()
                    .add_messages(messages)
                    .add_attribute("method", "split"))
            }
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::msg::{Config, OptionalServiceConfig};

    pub fn update_config(
        deps: &DepsMut,
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
        QueryMsg::GetOwner {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetServiceConfig {} => {
            let config: Config = valence_service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
    }
}

#[cfg(test)]
mod tests {}
