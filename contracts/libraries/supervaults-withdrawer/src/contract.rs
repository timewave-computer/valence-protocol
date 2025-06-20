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
        coin, ensure, to_json_binary, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, WasmMsg,
    };
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};
    use valence_supervaults_utils::prec_dec_range::PrecDecimalRange;

    use crate::msg::{Config, FunctionMsgs};

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

        // construct the supervaults withdraw message
        let withdraw_msg = WasmMsg::Execute {
            contract_addr: cfg.vault_addr.to_string(),
            msg: to_json_binary(&valence_supervaults_utils::msg::get_mmvault_withdraw_msg(
                input_acc_lp_bal.amount,
            ))?,
            funds: vec![input_acc_lp_bal.clone()],
        }
        .into();

        // Check how much we are going to receive by simulating the withdraw
        let (amount_asset1, amount_asset2) =
            valence_supervaults_utils::queries::query_simulate_withdraw_liquidity(
                deps.as_ref(),
                cfg.vault_addr.to_string(),
                input_acc_lp_bal.amount,
            )?;

        // Construct the array of coins to be sent from the input account to the output account
        // If for some reason any of the amounts is zero, we do not include it in the withdraw assets
        // Shouldn't happen, but it's a sanity check
        let withdrawn_assets: Vec<Coin> = [
            (amount_asset1, cfg.lw_config.asset_data.asset1),
            (amount_asset2, cfg.lw_config.asset_data.asset2),
        ]
        .into_iter()
        .filter_map(|(amount, asset)| (!amount.is_zero()).then(|| coin(amount.u128(), asset)))
        .collect();

        ensure!(
            !withdrawn_assets.is_empty(),
            LibraryError::ExecutionError("Nothing is being withdrawn!".to_string())
        );

        // Construct the bank message to transfer the withdrawn assets to the output account
        let transfer_assets_msg = BankMsg::Send {
            to_address: cfg.output_addr.to_string(),
            amount: withdrawn_assets.clone(),
        }
        .into();

        // Execute both messages on behalf of input_addr
        let execute_msg =
            execute_on_behalf_of(vec![withdraw_msg, transfer_assets_msg], &cfg.input_addr)?;

        Ok(Response::new()
            .add_message(execute_msg)
            .add_attribute("method", "withdraw_liquidity"))
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
