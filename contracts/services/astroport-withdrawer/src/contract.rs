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

mod actions {
    use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response};
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::{
        astroport_cw20, astroport_native,
        msg::{ActionsMsgs, Config, PoolType},
    };

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionsMsgs::WithdrawLiquidity {} => withdraw_liquidity(deps, cfg),
        }
    }

    fn withdraw_liquidity(deps: DepsMut, cfg: Config) -> Result<Response, ServiceError> {
        let msgs = create_withdraw_liquidity_msgs(&deps, &cfg)?;

        let input_account_msgs = execute_on_behalf_of(msgs, &cfg.input_addr)?;

        Ok(Response::new()
            .add_message(input_account_msgs)
            .add_attribute("method", "withdraw_liquidity"))
    }

    fn create_withdraw_liquidity_msgs(
        deps: &DepsMut,
        cfg: &Config,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        match &cfg.withdrawer_config.pool_type {
            PoolType::NativeLpToken => astroport_native::create_withdraw_liquidity_msgs(deps, cfg),
            PoolType::Cw20LpToken => astroport_cw20::create_withdraw_liquidity_msgs(deps, cfg),
        }
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
