#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_library_utils::{
    error::LibraryError,
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
    valence_library_base::execute(
        deps,
        env,
        info,
        msg,
        functions::process_function,
        execute::update_config,
    )
}

mod functions {
    use cosmwasm_std::{to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};
    use valence_lending_utils::mars::ActionCoin;
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::msg::{Config, FunctionMsgs};

    pub fn process_function(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::Withdraw {} => {
                // Query for the created credit account
                let acc_ids: Vec<valence_lending_utils::mars::Account> =
                    deps.querier.query_wasm_smart(
                        cfg.credit_manager_addr.to_string(),
                        &valence_lending_utils::mars::QueryMsg::Accounts {
                            owner: cfg.input_addr.to_string(),
                            start_after: None,
                            limit: None,
                        },
                    )?;

                // Valence account owns just one credit account
                let credit_acc = acc_ids.first().ok_or_else(|| {
                    LibraryError::ExecutionError("No credit account found".to_string())
                })?;

                // Prepare withdraw message
                let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.credit_manager_addr.to_string(),
                    msg: to_json_binary(
                        &valence_lending_utils::mars::ExecuteMsg::UpdateCreditAccount {
                            account_id: Some(credit_acc.id.clone()),
                            account_kind: Some(valence_lending_utils::mars::AccountKind::Default),
                            actions: vec![
                                valence_lending_utils::mars::Action::Reclaim(ActionCoin {
                                    denom: cfg.denom.clone(),
                                    amount:
                                        valence_lending_utils::mars::ActionAmount::AccountBalance,
                                }),
                                valence_lending_utils::mars::Action::WithdrawToWallet { coin: ActionCoin {
                                    denom: cfg.denom.clone(),
                                    amount:
                                        valence_lending_utils::mars::ActionAmount::AccountBalance,
                                }, recipient: cfg.output_addr.to_string() },
                            ],
                        },
                    )?,
                    funds: vec![],
                });

                // Execute on behalf of input_addr
                let execute_msg = execute_on_behalf_of(vec![withdraw_msg], &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(execute_msg)
                    .add_attribute("method", "withdraw")
                    .add_attribute("account_id", credit_acc.id.clone())
                    .add_attribute("owner", cfg.input_addr.to_string()))
            }
        }
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_library_utils::error::LibraryError;

    use crate::msg::LibraryConfigUpdate;

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        new_config: LibraryConfigUpdate,
    ) -> Result<(), LibraryError> {
        new_config.update_config(deps)
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
