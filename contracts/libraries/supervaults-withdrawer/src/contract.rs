#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult,
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
const LW_REPLY_ID: u64 = 1414;

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

mod functions {

    use cosmwasm_std::{
        ensure, to_json_binary, to_json_string, DepsMut, Env, MessageInfo, Response, SubMsg,
        WasmMsg,
    };
    use valence_library_utils::{error::LibraryError, execute_submsgs_on_behalf_of};
    use valence_supervaults_utils::prec_dec_range::PrecDecimalRange;

    use crate::{
        contract::LW_REPLY_ID,
        msg::{Config, FunctionMsgs},
    };

    pub fn process_function(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: FunctionMsgs,
        cfg: Config,
    ) -> Result<Response, LibraryError> {
        match msg {
            FunctionMsgs::WithdrawLiquidity {
                expected_vault_ratio_range,
            } => try_withdraw_liquidity(deps, cfg, expected_vault_ratio_range),
        }
    }

    fn try_withdraw_liquidity(
        deps: DepsMut,
        cfg: Config,
        expected_vault_ratio_range: Option<PrecDecimalRange>,
    ) -> Result<Response, LibraryError> {
        // if expected vault ratio range is specified, we validate it
        if let Some(range) = expected_vault_ratio_range {
            // query the current vault price
            let vault_price = valence_supervaults_utils::queries::query_vault_price(
                deps.as_ref(),
                cfg.vault_addr.to_string(),
            )?;
            // validate the query result against the specified range
            range.ensure_contains(vault_price)?;
        }

        // assert that the input account has available lp shares in their balance
        let input_acc_lp_bal = deps.querier.query_balance(
            cfg.input_addr.to_string(),
            cfg.lw_config.lp_denom.to_string(),
        )?;

        ensure!(
            !input_acc_lp_bal.amount.is_zero(),
            LibraryError::ExecutionError(
                "input account must have lp shares in order to withdraw".to_string()
            )
        );

        // construct the supervaults withdraw message
        let supervaults_withdraw_msg = mmvault::msg::ExecuteMsg::Withdraw {
            amount: input_acc_lp_bal.amount,
        };
        let withdraw_msg = WasmMsg::Execute {
            contract_addr: cfg.vault_addr.to_string(),
            msg: to_json_binary(&supervaults_withdraw_msg)?,
            funds: vec![input_acc_lp_bal],
        };

        // delegate the supervaults withdraw request to the input account
        // as a submessage
        let delegated_input_account_submsgs = execute_submsgs_on_behalf_of(
            vec![SubMsg::reply_on_success(withdraw_msg, LW_REPLY_ID)],
            Some(to_json_string(&cfg)?),
            &cfg.input_addr,
        )?;

        Ok(Response::new().add_submessage(SubMsg::reply_on_success(
            delegated_input_account_submsgs,
            LW_REPLY_ID,
        )))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        LW_REPLY_ID => {
            // extract configuration from the reply payload
            let cfg: Config = valence_account_utils::msg::parse_valence_payload(&msg.result)?;

            // query account resulting asset balance
            let asset1_balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), cfg.lw_config.asset_data.asset1)?;
            let asset2_balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), cfg.lw_config.asset_data.asset2)?;

            // filter out zero-amount balances
            let available_assets: Vec<Coin> = [asset1_balance, asset2_balance]
                .iter()
                .filter_map(|c| match c.amount.is_zero() {
                    true => None,
                    false => Some(c.clone()),
                })
                .collect();

            ensure!(
                !available_assets.is_empty(),
                LibraryError::ExecutionError("no available assets".to_string())
            );

            // construct the resulting asset transfer message to the output account
            let asset_transfer_msg = BankMsg::Send {
                to_address: cfg.output_addr.to_string(),
                amount: available_assets,
            };

            let delegated_msg =
                execute_on_behalf_of(vec![asset_transfer_msg.into()], &cfg.input_addr)?;

            Ok(Response::default().add_message(delegated_msg))
        }
        _ => Err(LibraryError::ExecutionError("unknown reply id".to_string())),
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
