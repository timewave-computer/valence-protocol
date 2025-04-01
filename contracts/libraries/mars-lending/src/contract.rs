use crate::msg::{Config, FunctionMsgs, LibraryConfig, LibraryConfigUpdate, QueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, to_json_string, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, SubMsgResult, WasmMsg,
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
const REPLY_ID: u64 = 1;

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
            // Query input account balance
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), cfg.denom.clone())?;

            if balance.amount.is_zero() {
                return Err(LibraryError::ExecutionError("No funds to lend".to_string()));
            }

            // Transfer funds to the output account for later deposit into the credit account
            let transfer_funds_msg = BankMsg::Send {
                to_address: cfg.output_addr.to_string(),
                amount: vec![balance],
            };

            let delegated_input_acc_msgs =
                execute_on_behalf_of(vec![transfer_funds_msg.into()], &cfg.input_addr.clone())?;

            // Create credit account creation message
            let create_credit_acc_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.credit_manager_addr.to_string(),
                msg: to_json_binary(
                    &valence_lending_utils::mars::ExecuteMsg::CreateCreditAccount(
                        valence_lending_utils::mars::AccountKind::Default,
                    ),
                )?,
                funds: vec![],
            });

            // Delegate the create credit account message to the output account with reply
            let delegated_output_acc_msgs = execute_submsgs_on_behalf_of(
                vec![SubMsg::reply_on_success(create_credit_acc_msg, REPLY_ID)],
                Some(to_json_string(&cfg)?),
                &cfg.output_addr, // make output account owner of the credit account
            )?;

            Ok(Response::new()
                .add_message(delegated_input_acc_msgs)
                .add_submessage(SubMsg::reply_on_success(
                    delegated_output_acc_msgs,
                    REPLY_ID,
                ))
                .add_attribute("method", "create_credit_account"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        REPLY_ID => handle_create_credit_account_reply(deps, msg.result),
        _ => Err(LibraryError::Std(StdError::generic_err("unknown reply id"))),
    }
}

fn handle_create_credit_account_reply(
    deps: DepsMut,
    result: SubMsgResult,
) -> Result<Response, LibraryError> {
    // Restore the configuration from the reply payload
    let cfg: Config = valence_account_utils::msg::parse_valence_payload(&result)?;

    // Query for the created credit account
    let credit_accounts: Vec<valence_lending_utils::mars::Account> =
        deps.querier.query_wasm_smart(
            cfg.credit_manager_addr.to_string(),
            &valence_lending_utils::mars::QueryMsg::Accounts {
                owner: cfg.output_addr.to_string(),
                start_after: None,
                limit: None,
            },
        )?;

    // Valence account owns just one credit account 
    let credit_account = credit_accounts
        .first()
        .ok_or_else(|| LibraryError::ExecutionError("No credit account found".to_string()))?;

    // Query account balance
    let balance = deps
        .querier
        .query_balance(cfg.output_addr.clone(), cfg.denom.clone())?;

    if balance.amount.is_zero() {
        return Err(LibraryError::ExecutionError("No funds to lend".to_string()));
    }

    // Prepare lending message
    let lend_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.credit_manager_addr.to_string(),
        msg: to_json_binary(
            &valence_lending_utils::mars::ExecuteMsg::UpdateCreditAccount {
                account_id: Some(credit_account.id.clone()),
                account_kind: Some(valence_lending_utils::mars::AccountKind::Default),
                actions: vec![
                    valence_lending_utils::mars::Action::Deposit(balance.clone()),
                    valence_lending_utils::mars::Action::Lend(
                        valence_lending_utils::mars::ActionCoin {
                            denom: cfg.denom.clone(),
                            amount: valence_lending_utils::mars::ActionAmount::AccountBalance,
                        },
                    ),
                ],
            },
        )?,
        funds: vec![balance.clone()],
    });

    // Execute on behalf of output_addr
    let execute_msg = execute_on_behalf_of(vec![lend_msg], &cfg.output_addr)?;

    Ok(Response::new()
        .add_message(execute_msg)
        .add_attribute("method", "lend")
        .add_attribute("account_id", credit_account.id.clone())
        .add_attribute("owner", cfg.output_addr.to_string())
        .add_attribute("amount", balance.to_string()))
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
