#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
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
const LP_REPLY_ID: u64 = 314;

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
        ensure, to_json_binary, to_json_string, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
        Response, SubMsg, WasmMsg,
    };
    use neutron_std::types::neutron::util::precdec::PrecDec;
    use valence_library_utils::{error::LibraryError, execute_submsgs_on_behalf_of};

    use crate::{
        contract::LP_REPLY_ID,
        msg::{Config, FunctionMsgs, PrecDecimalRange},
    };

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
        let (balance_asset1, balance_asset2) = query_asset_balances(&deps, &cfg)?;

        // filter out zero-amount balances
        let provision_assets: Vec<Coin> = [balance_asset1, balance_asset2]
            .iter()
            .filter_map(|c| match c.amount.is_zero() {
                true => None,
                false => Some(c.clone()),
            })
            .collect();

        // ensure that the input account has the necessary funds for liquidity provision
        ensure!(
            !provision_assets.is_empty(),
            LibraryError::ExecutionError(
                "liquidity provision requires at least one input denom".to_string()
            )
        );

        // validate the expected price
        let vault_price = query_vault_price(deps.as_ref(), cfg.vault_addr.to_string())?;
        if let Some(range) = expected_vault_ratio_range {
            ensure!(
                vault_price.ge(&range.min) && vault_price.lt(&range.max),
                LibraryError::ExecutionError(format!(
                    "expected range: {range}, got: {vault_price}"
                ))
            )
        }

        // construct lp message
        let supervaults_deposit_msg = mmvault::msg::ExecuteMsg::Deposit {};
        let provide_liquidity_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: cfg.vault_addr.to_string(),
            msg: to_json_binary(&supervaults_deposit_msg)?,

            funds: provision_assets,
        }
        .into();

        // delegate the LP submessage to the input account because supervault LP
        // shares get issued at an unknown rate.
        // to deal with that, we LP as a submessage and handle the resulting share
        // transfer from input acc to output acc in the response
        let delegated_input_account_submsgs = execute_submsgs_on_behalf_of(
            vec![SubMsg::reply_on_success(provide_liquidity_msg, LP_REPLY_ID)],
            Some(to_json_string(&cfg)?),
            &cfg.input_addr,
        )?;

        Ok(Response::new().add_submessage(SubMsg::reply_on_success(
            delegated_input_account_submsgs,
            LP_REPLY_ID,
        )))
    }

    fn query_asset_balances(deps: &DepsMut, cfg: &Config) -> Result<(Coin, Coin), LibraryError> {
        let balance_asset1 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset1)?;
        let balance_asset2 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset2)?;
        Ok((balance_asset1, balance_asset2))
    }

    fn query_vault_price(deps: Deps, vault_addr: String) -> Result<PrecDec, LibraryError> {
        let price_response: mmvault::msg::CombinedPriceResponse = deps
            .querier
            .query_wasm_smart(vault_addr, &mmvault::msg::QueryMsg::GetPrices {})?;

        Ok(price_response.price_0_to_1)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, LibraryError> {
    match msg.id {
        LP_REPLY_ID => {
            // extract configuration from the reply payload
            let cfg: Config = valence_account_utils::msg::parse_valence_payload(&msg.result)?;

            // query account resulting LP balance
            let balance = deps
                .querier
                .query_balance(cfg.input_addr.clone(), cfg.lp_config.lp_denom.clone())?;

            ensure!(
                !balance.amount.is_zero(),
                LibraryError::ExecutionError("input account shares balance is zero".to_string())
            );

            // construct the resulting share transfer message to the output account
            let lp_share_transfer_msg = BankMsg::Send {
                to_address: cfg.output_addr.to_string(),
                amount: vec![balance],
            };

            let delegated_msg =
                execute_on_behalf_of(vec![lp_share_transfer_msg.into()], &cfg.input_addr)?;

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
