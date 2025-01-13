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
    use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response};
    use valence_astroport_utils::{
        decimal_range::DecimalRange, get_pool_asset_amounts, query_pool, PoolType,
    };
    use valence_library_utils::{error::LibraryError, execute_on_behalf_of};

    use crate::{
        astroport_cw20, astroport_native,
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
                expected_pool_ratio_range,
            } => withdraw_liquidity(deps, cfg, expected_pool_ratio_range),
        }
    }

    fn withdraw_liquidity(
        deps: DepsMut,
        cfg: Config,
        expected_pool_ratio_range: Option<DecimalRange>,
    ) -> Result<Response, LibraryError> {
        // If we have an expected pool ratio range, we need to check if the pool is within that range
        if let Some(range) = expected_pool_ratio_range {
            // Get assets in the pool
            let pool_response = query_pool(
                &deps,
                cfg.pool_addr.as_ref(),
                &cfg.withdrawer_config.pool_type,
            )?;

            // Get the amounts of each of the assets of our config in the pool
            let (pool_asset1_balance, pool_asset2_balance) = get_pool_asset_amounts(
                pool_response,
                &cfg.withdrawer_config.asset_data.asset1,
                &cfg.withdrawer_config.asset_data.asset2,
            )?;

            // Get the pool asset ratios
            let pool_asset_ratios =
                cosmwasm_std::Decimal::checked_from_ratio(pool_asset1_balance, pool_asset2_balance)
                    .map_err(|e| LibraryError::ExecutionError(e.to_string()))?;

            range.is_within_range(pool_asset_ratios)?;
        }

        let msgs = create_withdraw_liquidity_msgs(&deps, &cfg)?;

        let input_account_msgs = execute_on_behalf_of(msgs, &cfg.input_addr)?;

        Ok(Response::new()
            .add_message(input_account_msgs)
            .add_attribute("method", "withdraw_liquidity"))
    }

    fn create_withdraw_liquidity_msgs(
        deps: &DepsMut,
        cfg: &Config,
    ) -> Result<Vec<CosmosMsg>, LibraryError> {
        match &cfg.withdrawer_config.pool_type {
            PoolType::NativeLpToken(_) => {
                astroport_native::create_withdraw_liquidity_msgs(deps, cfg)
            }
            PoolType::Cw20LpToken(_) => astroport_cw20::create_withdraw_liquidity_msgs(deps, cfg),
        }
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
