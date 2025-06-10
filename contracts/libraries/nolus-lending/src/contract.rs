use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use valence_library_utils::{
    error::LibraryError,
    execute_on_behalf_of, execute_submsgs_on_behalf_of,
    msg::{ExecuteMsg, InstantiateMsg},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Unique ID for reply handling
const WITHDRAW_REPLY_ID: u64 = 1;

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

pub fn process_function(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
    cfg: Config,
) -> Result<Response, LibraryError> {
    match msg {
        FunctionMsgs::Lend {} => {
            // Query account balance
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), cfg.denom.clone())?;

            if balance.amount.is_zero() {
                return Err(LibraryError::ExecutionError("No funds to lend".to_string()));
            }

            // Prepare lend message
            let lend_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.pool_addr.to_string(),
                msg: to_json_binary(&valence_lending_utils::nolus::ExecuteMsg::Deposit {})?,
                funds: vec![balance.clone()],
            });

            // Execute on behalf of input_addr
            let execute_msg = execute_on_behalf_of(vec![lend_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(execute_msg)
                .add_attribute("method", "lend")
                .add_attribute("amount", balance.to_string())
                .add_attribute("input_addr", cfg.input_addr.to_string()))
        }
        FunctionMsgs::Withdraw { amount } => {
            // Check the nlp balance of the input address (lender)
            let balance_nlpn_resp: valence_lending_utils::nolus::BalanceResponse =
                deps.querier.query_wasm_smart(
                    cfg.pool_addr.to_string(),
                    &valence_lending_utils::nolus::QueryMsg::Balance {
                        address: cfg.input_addr.clone(),
                    },
                )?;

            let balance_nlp: Uint128 = balance_nlpn_resp.balance;
            if balance_nlp.is_zero() {
                return Err(LibraryError::ExecutionError(
                    "No funds to withdraw".to_string(),
                ));
            }

            // Check withdrawal amount
            let withdrawal_amount = match amount {
                // withdraw exact amount
                Some(amt) => {
                    if amt > balance_nlp || amt.is_zero() {
                        return Err(LibraryError::ExecutionError(
                            "Withdraw amount is either zero or bigger than balance".to_string(),
                        ));
                    }
                    amt
                }
                // if no amount is specified, we withdraw the entire position
                None => balance_nlp,
            };

            // Prepare withdraw message
            let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.pool_addr.to_string(),
                msg: to_json_binary(&valence_lending_utils::nolus::ExecuteMsg::Burn {
                    amount: withdrawal_amount,
                })?,
                funds: vec![],
            });

            // Execute on behalf of input_addr with reply. On reply we will send the funds to the output address
            let execute_msg = execute_submsgs_on_behalf_of(
                vec![SubMsg::reply_on_success(withdraw_msg, WITHDRAW_REPLY_ID)],
                Some(to_json_string(&cfg)?),
                &cfg.input_addr,
            )?;

            Ok(Response::new()
                .add_submessage(SubMsg::reply_on_success(execute_msg, WITHDRAW_REPLY_ID))
                .add_attribute("method", "burn"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        WITHDRAW_REPLY_ID => {
            // Extract configuration from the reply payload
            let cfg: Config = valence_account_utils::msg::parse_valence_payload(&msg.result)?;

            // Query account balance of input account after withdrawal
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), cfg.denom.clone())?;

            if balance.amount.is_zero() {
                return Err(LibraryError::ExecutionError(
                    "No withdrawn funds".to_string(),
                ));
            }

            // Transfer the withdrawn funds to the output address
            let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: cfg.output_addr.to_string(),
                amount: vec![balance.clone()],
            });

            let execute_msg = execute_on_behalf_of(vec![send_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(execute_msg)
                .add_attribute("method", "withdraw")
                .add_attribute("amount", balance.to_string())
                .add_attribute("output_addr", cfg.output_addr.to_string()))
        }
        _ => Err(LibraryError::Std(StdError::generic_err("unknown reply id"))),
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    new_config: LibraryConfigUpdate,
) -> Result<(), LibraryError> {
    new_config.update_config(deps)
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
