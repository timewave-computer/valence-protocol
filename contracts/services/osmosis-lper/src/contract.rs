#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::{
    msg::{ActionsMsgs, QueryMsg},
    valence_service_integration::{Config, OptionalServiceConfig, ServiceConfig},
};

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

    use crate::valence_service_integration::{Config, OptionalServiceConfig};

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
    use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};

    use valence_service_utils::error::ServiceError;

    use crate::{msg::ActionsMsgs, pool_types::balancer, valence_service_integration::Config};

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        println!("processing osmosis liquid pooler action: {:?}", msg);
        match msg {
            ActionsMsgs::ProvideDoubleSidedLiquidity {} => {
                provide_double_sided_liquidity(deps, cfg)
            }
            ActionsMsgs::ProvideSingleSidedLiquidity { asset, limit } => {
                provide_single_sided_liquidity(deps, cfg, asset, limit)
            }
        }
    }

    fn provide_single_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
        asset: String,
        limit: Uint128,
    ) -> Result<Response, ServiceError> {
        balancer::provide_single_sided_liquidity(deps, cfg, asset, limit)
    }

    fn provide_double_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        balancer::provide_double_sided_liquidity(deps, cfg)
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
