use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};
use cosmos_sdk_proto::traits::MessageExt;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg,
};
use valence_lending_utils::elys::{
    ElysQuery, MsgBond, MsgClaimRewards, MsgUnbond, QueryCommittedTokensLockedResponse,
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
const CLAIM_REPLY_ID: u64 = 2;

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
    deps: DepsMut<ElysQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<FunctionMsgs, LibraryConfigUpdate>,
) -> Result<Response, LibraryError> {
    valence_library_base::execute(deps, env, info, msg, process_function, update_config)
}

pub fn process_function(
    deps: DepsMut<ElysQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: FunctionMsgs,
    cfg: Config,
) -> Result<Response, LibraryError> {
    match msg {
        FunctionMsgs::Lend {} => {
            // Query the pool to get the deposit denom
            let pool = valence_lending_utils::elys::query_pool(&deps, cfg.pool_id.u64())?;

            // Query account balance
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), pool.deposit_denom.clone())?;

            if balance.amount.is_zero() {
                return Err(LibraryError::ExecutionError("No funds to lend".to_string()));
            }

            // Prepare lend message
            let lend_cosmos_msg = MsgBond {
                amount: balance.clone().amount.to_string(),
                pool_id: cfg.pool_id.into(),
                creator: cfg.input_addr.to_string(),
            };

            #[allow(deprecated)]
            let lend_msg = CosmosMsg::Stargate {
                type_url: "/elys.stablestake.MsgBond".to_string(),
                value: Binary::from(lend_cosmos_msg.to_bytes().unwrap()),
            };

            // Execute on behalf of input_addr
            let execute_msg = execute_on_behalf_of(vec![lend_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(execute_msg)
                .add_attribute("method", "lend")
                .add_attribute("amount", balance.to_string())
                .add_attribute("input_addr", cfg.input_addr.to_string()))
        }
        FunctionMsgs::Withdraw { amount } => {
            // Check the balance of the input address (lender) for locked tokens
            let query = ElysQuery::QueryCommittedTokensLocked {
                address: cfg.input_addr.to_string(),
            };
            let tokens_locked: QueryCommittedTokensLockedResponse =
                deps.querier.query(&query.into())?;

            // We know there will be only one locked coin because we are locking tokens specifically for this pool.
            // If no coins are found, it means there are no funds locked for the specified pool, and an error is returned.
            let balance_locked: Coin = tokens_locked
                .locked_committed
                .into_iter()
                .next() // Take the first coin
                .ok_or_else(|| {
                    LibraryError::ExecutionError(
                        "No funds locked for the specified denom".to_string(),
                    )
                })?;

            // Check if amount_locked is greater than zero
            if balance_locked.amount.is_zero() {
                return Err(LibraryError::ExecutionError(
                    "Available amount for withdrawal is zero".to_string(),
                ));
            }

            // Check withdrawal amount
            let withdrawal_amount = match amount {
                // withdraw exact amount
                Some(amt) => {
                    if amt > balance_locked.amount || amt.is_zero() {
                        return Err(LibraryError::ExecutionError(
                            "Withdraw amount is either zero or bigger than balance".to_string(),
                        ));
                    }
                    amt.to_string()
                }
                // if no amount is specified, we withdraw the entire position
                None => balance_locked.amount.to_string(),
            };

            // Prepare withdraw message
            let withdraw_cosmos_msg = MsgUnbond {
                amount: withdrawal_amount.to_string(),
                pool_id: cfg.pool_id.into(),
                creator: cfg.input_addr.to_string(),
            };

            #[allow(deprecated)]
            let withdraw_msg = CosmosMsg::Stargate {
                type_url: "/elys.stablestake.MsgUnbond".to_string(),
                value: Binary::from(withdraw_cosmos_msg.to_bytes().unwrap()),
            };

            // Execute on behalf of input_addr with reply. On reply we will send the funds to the output address
            let execute_msg = execute_submsgs_on_behalf_of(
                vec![SubMsg::reply_on_success(withdraw_msg, WITHDRAW_REPLY_ID)],
                Some(to_json_string(&cfg)?),
                &cfg.input_addr,
            )?;

            Ok(Response::new()
                .add_submessage(SubMsg::reply_on_success(execute_msg, WITHDRAW_REPLY_ID)))
        }
        FunctionMsgs::ClaimRewards {} => {
            // Prepare claim message
            let claim_cosmos_msg = MsgClaimRewards {
                sender: cfg.input_addr.to_string(),
                pool_ids: vec![cfg.pool_id.into()],
            };
            #[allow(deprecated)]
            let claim_msg = CosmosMsg::Stargate {
                type_url: "/elys.masterchef.MsgClaimRewards".to_string(),
                value: Binary::from(claim_cosmos_msg.to_bytes().unwrap()),
            };
            // Execute on behalf of input_addr with reply to handle reward transfer
            let execute_msg = execute_submsgs_on_behalf_of(
                vec![SubMsg::reply_on_success(claim_msg, CLAIM_REPLY_ID)],
                Some(to_json_string(&cfg)?),
                &cfg.input_addr,
            )?;
            Ok(Response::new()
                .add_submessage(SubMsg::reply_on_success(execute_msg, CLAIM_REPLY_ID)))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut<ElysQuery>, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        WITHDRAW_REPLY_ID => {
            // Extract configuration from the reply payload
            let cfg: Config = valence_account_utils::msg::parse_valence_payload(&msg.result)?;

            // Query the pool to get the deposit denom
            let pool = valence_lending_utils::elys::query_pool(&deps, cfg.pool_id.u64())?;

            // Query account balance of input account after withdrawal
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), pool.deposit_denom.clone())?;

            if balance.amount.is_zero() {
                return Err(LibraryError::ExecutionError(
                    "No funds to withdraw".to_string(),
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
        CLAIM_REPLY_ID => {
            // Extract configuration from the reply payload
            let cfg: Config = valence_account_utils::msg::parse_valence_payload(&msg.result)?;

            // Query account balances of input account
            #[allow(deprecated)]
            let balances = deps
                .querier
                .query_all_balances(cfg.input_addr.clone())
                .unwrap();

            // Transfer the claimed rewards to the output address
            let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: cfg.output_addr.to_string(),
                amount: balances.clone(),
            });

            let execute_msg = execute_on_behalf_of(vec![send_msg], &cfg.input_addr)?;

            Ok(Response::new()
                .add_message(execute_msg)
                .add_attribute("method", "claim_rewards")
                .add_attribute("output_addr", cfg.output_addr.to_string()))
        }
        _ => Err(LibraryError::Std(StdError::generic_err("unknown reply id"))),
    }
}

pub fn update_config(
    deps: DepsMut<ElysQuery>,
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
