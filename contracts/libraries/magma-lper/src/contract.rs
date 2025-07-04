#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};
use valence_library_utils::{
    error::LibraryError,
    execute_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<LibraryConfig>,
) -> Result<Response, LibraryError> {
    valence_library_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(deps, env, info, msg, process_function, update_config)
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    new_config: LibraryConfigUpdate,
) -> Result<(), LibraryError> {
    new_config.update_config(deps)
}

pub fn process_function(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
    cfg: Config,
) -> Result<Response, LibraryError> {
    match msg {
        FunctionMsgs::ProvideLiquidity {
            token_min_amount_0,
            token_min_amount_1,
        } => {
            let deposit_msg = valence_magma_utils::msg::DepositMsg {
                amount0_min: token_min_amount_0.unwrap_or_default(),
                amount1_min: token_min_amount_1.unwrap_or_default(),
                to: cfg.output_addr.to_string(),
            };

            let bal_asset_0 = deps
                .querier
                .query_balance(&cfg.input_addr, cfg.lp_config.asset_data.asset1.as_str())?;
            let bal_asset_1 = deps
                .querier
                .query_balance(&cfg.input_addr, cfg.lp_config.asset_data.asset2.as_str())?;

            let cosmos_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_config.vault_addr.to_string(),
                msg: to_json_binary(&deposit_msg)?,
                funds: vec![bal_asset_0.clone(), bal_asset_1.clone()],
            });

            let delegate_msg = execute_on_behalf_of(vec![cosmos_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(delegate_msg)
                .add_attribute("method", "deposit")
                .add_attribute("asset_1", bal_asset_0.to_string())
                .add_attribute("asset_2", bal_asset_1.to_string()))
        }
    }
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_library_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_library_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetLibraryConfig {} => {
            let config: Config = valence_library_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
        QueryMsg::GetRawLibraryConfig {} => {
            let raw_config: LibraryConfig =
                valence_library_utils::raw_config::query_raw_library_config(deps.storage)?;
            to_json_binary(&raw_config)
        }
    }
}
