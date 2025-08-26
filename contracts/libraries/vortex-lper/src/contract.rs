#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    from_json, to_json_binary, to_json_vec, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, WasmMsg,
};
use valence_library_utils::{
    error::LibraryError,
    execute_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};
use valence_osmosis_utils::utils::cl_utils::query_cl_pool;
use valence_vortex_utils::msg::CreatePositionMsg;

use crate::{
    msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg, ReplyPayload},
    state::VORTEX_CONTRACT_ADDR,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REPLY_ID_INSTANTIATE_VORTEX: u64 = 1;

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
    env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
    cfg: Config,
) -> Result<Response, LibraryError> {
    match msg {
        FunctionMsgs::ProvideLiquidity {
            tick_range,
            principal_token_min_amount,
            counterparty_token_min_amount,
        } => {
            // Check if vortex contract already exists
            if VORTEX_CONTRACT_ADDR.may_load(deps.storage)?.is_some() {
                return Err(LibraryError::Std(StdError::generic_err(
                    "Vortex contract already exists",
                )));
            }

            let instantiate_msg = valence_vortex_utils::msg::InstantiateMsg {
                pool_id: cfg.lp_config.pool_id,
                principal_denom: cfg.lp_config.asset_data.asset1.clone(),
                counterparty_denom: cfg.lp_config.asset_data.asset2.clone(),
                round_duration: cfg.lp_config.round_duration,
                position_admin: Some(cfg.input_addr.clone().to_string()),
                counterparty_owner: Some(cfg.output_addr_2.clone().to_string()),
                principal_funds_owner: cfg.output_addr.clone().to_string(),
                auction_duration: cfg.lp_config.auction_duration,
                principal_first: cfg.lp_config.principal_first,
            };

            let instantiate_msg_bin = to_json_binary(&instantiate_msg)?;

            let instantiate_cosmos_msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: None,
                code_id: cfg.lp_config.vortex_code,
                msg: instantiate_msg_bin,
                funds: vec![],
                label: cfg.lp_config.label.clone(),
            });

            // first we validate the custom target range
            tick_range.validate()?;

            // query the pool config
            let pool_cfg = query_cl_pool(&deps.as_ref(), cfg.lp_config.pool_id)?;

            // the target range must respect the pool tick spacing configuration
            tick_range.ensure_pool_spacing_compatibility(&pool_cfg)?;

            // Construct the payload
            let payload = valence_vortex_utils::msg::CreatePositionMsg {
                lower_tick: tick_range.lower_tick.into(),
                upper_tick: tick_range.upper_tick.into(),
                principal_token_min_amount,
                counterparty_token_min_amount,
            };

            let reply_payload = ReplyPayload {
                config: cfg.clone(),
                create_position_msg: payload.clone(),
            };

            let inst_submsg =
                SubMsg::reply_on_success(instantiate_cosmos_msg, REPLY_ID_INSTANTIATE_VORTEX)
                    .with_payload(to_json_vec(&reply_payload)?);

            Ok(Response::default()
                .add_submessage(inst_submsg)
                .add_attribute("method", "deposit"))
        }
        FunctionMsgs::WithdrawLiquidity {} => {
            let execute_msg = valence_vortex_utils::msg::ExecuteMsg::EndRound {};

            let vortex_addr = VORTEX_CONTRACT_ADDR.load(deps.storage)?;

            let state_query = valence_vortex_utils::msg::QueryMsg::State {};

            // Query the contract state
            let state: valence_vortex_utils::msg::StateResponse = deps
                .querier
                .query_wasm_smart(vortex_addr.clone(), &state_query)?;

            // Compare current block time with end_round
            if env.block.time < state.round_end_time {
                return Err(LibraryError::ExecutionError(
                    "Current round has not ended yet".to_string(),
                ));
            }

            // Check that a position exists
            if state.position_id.is_none() {
                return Err(LibraryError::ExecutionError(
                    "No position found".to_string(),
                ));
            }

            let cosmos_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: vortex_addr,
                msg: to_json_binary(&execute_msg)?,
                funds: vec![],
            });

            let withdraw_msg = execute_on_behalf_of(vec![cosmos_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(withdraw_msg)
                .add_attribute("method", "withdraw"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        REPLY_ID_INSTANTIATE_VORTEX => {
            let reply_payload: ReplyPayload = from_json(&msg.payload)?;

            let cfg = reply_payload.config;

            let bytes = &msg
                .result
                .into_result()
                .map_err(|e| LibraryError::Std(StdError::generic_err(e)))?
                .msg_responses[0]
                .clone()
                .value
                .to_vec();

            let instantiate_msg_response = cw_utils::parse_instantiate_response_data(bytes)
                .map_err(|e| {
                    StdError::generic_err(format!("failed to parse reply message: {e:?}"))
                })?;

            let create_position_msg: CreatePositionMsg = reply_payload.create_position_msg;

            VORTEX_CONTRACT_ADDR.save(
                deps.storage,
                &instantiate_msg_response.contract_address.clone(),
            )?;

            let create_position_msg = valence_vortex_utils::msg::CreatePositionMsg {
                lower_tick: create_position_msg.lower_tick,
                upper_tick: create_position_msg.upper_tick,
                principal_token_min_amount: create_position_msg.principal_token_min_amount,
                counterparty_token_min_amount: create_position_msg.counterparty_token_min_amount,
            };

            let execute_msg =
                valence_vortex_utils::msg::ExecuteMsg::CreatePosition(create_position_msg);

            let bal_asset_0 = deps
                .querier
                .query_balance(&cfg.input_addr, cfg.lp_config.asset_data.asset1.as_str())?;
            let bal_asset_1 = deps
                .querier
                .query_balance(&cfg.input_addr, cfg.lp_config.asset_data.asset2.as_str())?;

            let mut funds = vec![bal_asset_0.clone(), bal_asset_1.clone()];
            funds.sort_by(|a, b| a.denom.cmp(&b.denom)); // lexicographical sort

            let cosmos_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: instantiate_msg_response.contract_address.clone(),
                msg: to_json_binary(&execute_msg)?,
                funds,
            });

            let delegate_msg = execute_on_behalf_of(vec![cosmos_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(delegate_msg)
                .add_attribute("action", "instantiate_vortex_and_create_position")
                .add_attribute(
                    "vortex_addr",
                    instantiate_msg_response.contract_address.clone(),
                ))
        }

        _ => Err(LibraryError::ExecutionError("Unknown reply ID".to_string())),
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
        QueryMsg::GetVortexAddress {} => {
            let addr = VORTEX_CONTRACT_ADDR.load(deps.storage)?;
            to_json_binary(&addr)
        }
    }
}
