#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use service_base::{
    msg::{ExecuteMsg, InstantiateMsg},
    ServiceError,
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
    service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionsMsgs, OptionalServiceConfig>,
) -> Result<Response, ServiceError> {
    service_base::execute(
        deps,
        env,
        info,
        msg,
        actions::process_action,
        execute::update_config,
    )
}

mod actions {
    use base_account::msg::execute_on_behalf_of;
    use cosmwasm_std::{coin, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response};
    use service_base::ServiceError;

    use crate::msg::{ActionsMsgs, Config};

    pub fn process_action(
        deps: &DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionsMsgs::Forward { execution_id: _ } => {
                let coins_to_transfer: Vec<_> = cfg
                    .forwarding_configs()
                    .iter()
                    .filter_map(|(denom, fwd_cfg)| {
                        deps.querier
                            .query_balance(cfg.input_addr(), denom)
                            .ok()
                            .filter(|balance| !balance.amount.is_zero())
                            .map(|balance| {
                                let amount_to_transfer = balance.amount.min(*fwd_cfg.max_amount());
                                coin(amount_to_transfer.into(), denom)
                            })
                    })
                    .collect();

                let bank_sends: Vec<CosmosMsg> = coins_to_transfer
                    .into_iter()
                    .map(|c| {
                        BankMsg::Send {
                            to_address: cfg.output_addr().to_string(),
                            amount: vec![c],
                        }
                        .into()
                    })
                    .collect();
                let input_account_msgs =
                    execute_on_behalf_of(bank_sends, &cfg.input_addr().clone().into())?;

                Ok(Response::new()
                    .add_attribute("method", "forward")
                    .add_message(input_account_msgs))
            }
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use service_base::ServiceError;

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
        QueryMsg::GetProcessor {} => to_json_binary(&service_base::get_processor(deps.storage)?),
        QueryMsg::GetServiceConfig {} => {
            let config: Config = service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
    }
}
