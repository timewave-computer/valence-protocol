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
        coins, ensure, to_json_binary, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, WasmMsg,
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
            FunctionMsgs::ProvideLiquidity {
                expected_vault_ratio_range,
            } => try_provide_liquidity(deps, cfg, expected_vault_ratio_range),
        }
    }

    fn try_provide_liquidity(
        deps: DepsMut,
        cfg: Config,
        expected_vault_ratio_range: Option<PrecDecimalRange>,
    ) -> Result<Response, LibraryError> {
        // query the input account pool asset balances
        let balance_asset1 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset1)?;
        let balance_asset2 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset2)?;

        // filter out zero-amount balances
        let provision_assets: Vec<Coin> = [balance_asset1.clone(), balance_asset2.clone()]
            .into_iter()
            .filter(|c| !c.amount.is_zero())
            .collect();

        // ensure that the input account has the necessary funds for liquidity provision
        ensure!(
            !provision_assets.is_empty(),
            LibraryError::ExecutionError(
                "liquidity provision requires at least one input denom".to_string()
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

        // construct lp message
        let provide_liquidity_msg = WasmMsg::Execute {
            contract_addr: cfg.vault_addr.to_string(),
            msg: to_json_binary(&valence_supervaults_utils::msg::get_mmvault_deposit_msg())?,
            funds: provision_assets,
        }
        .into();

        // check how many shares we are going to get when providing liquidity
        let shares_amount = valence_supervaults_utils::queries::query_simulate_provide_liquidity(
            deps.as_ref(),
            cfg.vault_addr.to_string(),
            cfg.input_addr.clone(),
            balance_asset1.amount,
            balance_asset2.amount,
        )?;

        // create a bank message to transfer the shares to the output account
        let transfer_shares_msg = BankMsg::Send {
            to_address: cfg.output_addr.to_string(),
            amount: coins(shares_amount.u128(), cfg.lp_config.lp_denom),
        }
        .into();

        // Execute both messages on behalf of input_addr
        let execute_msg = execute_on_behalf_of(
            vec![provide_liquidity_msg, transfer_shares_msg],
            &cfg.input_addr,
        )?;

        Ok(Response::new()
            .add_message(execute_msg)
            .add_attribute("method", "provide_liquidity"))
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
