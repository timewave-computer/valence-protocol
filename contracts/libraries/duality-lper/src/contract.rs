#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, WasmMsg,
};
use valence_library_utils::{
    error::LibraryError,
    execute_on_behalf_of, execute_submsgs_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Unique ID for reply handling
const DEPOSIT_REPLY_ID: u64 = 1;

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
        FunctionMsgs::ProvideLiquidity {} => {
            let bal_asset_1 = deps
                .querier
                .query_balance(&cfg.input_addr, cfg.lp_config.asset_data.asset1.as_str())?;
            let bal_asset_2 = deps
                .querier
                .query_balance(&cfg.input_addr, cfg.lp_config.asset_data.asset2.as_str())?;

            let deposit_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.lp_config.pool_addr.to_string(),
                msg: to_json_binary(&valence_duality_utils::msg::ExecuteMsg::Deposit {})?,
                funds: vec![bal_asset_1.clone(), bal_asset_2.clone()],
            });

            // we delegate the position creation message to the input account
            let delegated_input_acc_msgs = execute_submsgs_on_behalf_of(
                vec![SubMsg::reply_on_success(deposit_msg, DEPOSIT_REPLY_ID)],
                Some(to_json_string(&cfg)?),
                &cfg.input_addr.clone(),
            )?;

            Ok(Response::default()
                .add_submessage(SubMsg::reply_on_success(
                    delegated_input_acc_msgs,
                    DEPOSIT_REPLY_ID,
                ))
                .add_attribute("method", "deposit")
                .add_attribute("asset_1", bal_asset_1.to_string())
                .add_attribute("asset_2", bal_asset_2.to_string()))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        DEPOSIT_REPLY_ID => {
            // Extract configuration from the reply payload
            let cfg: Config = valence_account_utils::msg::parse_valence_payload(&msg.result)?;

            // We need lp token denom from the pool config
            let pool_config: valence_duality_utils::utils::PoolConfig = deps.querier.query_wasm_smart(
                cfg.lp_config.pool_addr.clone(),
                &valence_duality_utils::msg::QueryMsg::GetConfig {},
            ).map_err(|e| LibraryError::ExecutionError(format!("Failed to query pool config: {}", e)))?;        

            // Query lp token balance of the input address
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), pool_config.lp_denom.clone())?;

            if balance.amount.is_zero() {
                return Err(LibraryError::ExecutionError(
                    "No deposited funds".to_string(),
                ));
            }

            // Transfer deposited funds to the output address
            let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: cfg.output_addr.to_string(),
                amount: vec![balance.clone()],
            });

            let execute_msg = execute_on_behalf_of(vec![send_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(execute_msg)
                .add_attribute("method", "provide_liquidity")
                .add_attribute("lp amount", balance.to_string())
                .add_attribute("output_addr", cfg.output_addr.to_string()))
        }
        _ => Err(LibraryError::Std(StdError::generic_err("unknown reply id"))),
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
