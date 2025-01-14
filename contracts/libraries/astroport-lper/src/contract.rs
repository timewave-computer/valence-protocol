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
    use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
    use valence_astroport_utils::{
        decimal_checked_ops::DecimalCheckedOps, decimal_range::DecimalRange,
        get_pool_asset_amounts, query_pool, PoolType,
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
            FunctionMsgs::ProvideDoubleSidedLiquidity {
                expected_pool_ratio_range,
            } => provide_double_sided_liquidity(deps, cfg, expected_pool_ratio_range),
            FunctionMsgs::ProvideSingleSidedLiquidity {
                asset,
                limit,
                expected_pool_ratio_range,
            } => provide_single_sided_liquidity(deps, cfg, asset, limit, expected_pool_ratio_range),
        }
    }

    fn provide_double_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
        expected_pool_ratio_range: Option<DecimalRange>,
    ) -> Result<Response, LibraryError> {
        // Get balances of both assets from input account
        let (balance_asset1, balance_asset2) = query_asset_balances(&deps, &cfg)?;
        // Get assets in the pool
        let pool_response = query_pool(&deps, cfg.pool_addr.as_ref(), &cfg.lp_config.pool_type)?;

        // Get the amounts of each of the assets of our config in the pool
        let (pool_asset1_balance, pool_asset2_balance) = get_pool_asset_amounts(
            pool_response,
            &cfg.lp_config.asset_data.asset1,
            &cfg.lp_config.asset_data.asset2,
        )?;

        // Get the pool asset ratios
        let pool_asset_ratios =
            cosmwasm_std::Decimal::checked_from_ratio(pool_asset1_balance, pool_asset2_balance)
                .map_err(|e| LibraryError::ExecutionError(e.to_string()))?;

        // If we have an expected pool ratio range, we need to check if the pool is within that range
        if let Some(range) = expected_pool_ratio_range {
            range.is_within_range(pool_asset_ratios)?;
        }

        let (asset1_provide_amount, asset2_provide_amount) = calculate_provide_amounts(
            balance_asset1.amount.u128(),
            balance_asset2.amount.u128(),
            pool_asset1_balance,
            pool_asset2_balance,
            pool_asset_ratios,
        )?;

        let cosmos_msg =
            create_provide_liquidity_msg(&cfg, asset1_provide_amount, asset2_provide_amount)?;

        let input_account_msgs = execute_on_behalf_of(vec![cosmos_msg], &cfg.input_addr)?;

        Ok(Response::new()
            .add_message(input_account_msgs)
            .add_attribute("method", "provide_double_sided_liquidity")
            .add_attribute("asset1_amount", asset1_provide_amount.to_string())
            .add_attribute("asset2_amount", asset2_provide_amount.to_string()))
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

    fn calculate_provide_amounts(
        balance1: u128,
        balance2: u128,
        pool_asset1_balance: u128,
        pool_asset2_balance: u128,
        pool_asset_ratio: cosmwasm_std::Decimal,
    ) -> Result<(u128, u128), LibraryError> {
        // Let's get the maximum amount of assets that we can provide liquidity
        let required_asset1_amount = pool_asset_ratio
            .checked_mul_uint128(balance2.into())
            .map_err(|error| LibraryError::ExecutionError(error.to_string()))?;

        // We can provide all asset2 tokens along with the corresponding maximum of asset1 tokens
        if balance1 >= required_asset1_amount.u128() {
            Ok((required_asset1_amount.u128(), balance2))
        } else {
            // We can't provide all asset2 tokens so we need to determine how many we can provide according to our available asset1
            let ratio =
                cosmwasm_std::Decimal::checked_from_ratio(pool_asset2_balance, pool_asset1_balance)
                    .map_err(|e| LibraryError::ExecutionError(e.to_string()))?;

            Ok((
                balance1,
                ratio
                    .checked_mul_uint128(balance1.into())
                    .map_err(|error| LibraryError::ExecutionError(error.to_string()))?
                    .u128(),
            ))
        }
    }

    fn create_provide_liquidity_msg(
        cfg: &Config,
        amount1: u128,
        amount2: u128,
    ) -> Result<CosmosMsg, LibraryError> {
        match &cfg.lp_config.pool_type {
            PoolType::NativeLpToken(_) => {
                astroport_native::create_provide_liquidity_msg(cfg, amount1, amount2)
            }
            PoolType::Cw20LpToken(_) => {
                astroport_cw20::create_provide_liquidity_msg(cfg, amount1, amount2)
            }
        }
    }

    fn provide_single_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
        asset: String,
        limit: Option<Uint128>,
        expected_pool_ratio_range: Option<DecimalRange>,
    ) -> Result<Response, LibraryError> {
        // Query asset balances and pool
        let (balance_asset1, balance_asset2) = query_asset_balances(&deps, &cfg)?;
        let pool_response = query_pool(&deps, cfg.pool_addr.as_ref(), &cfg.lp_config.pool_type)?;

        // Get pool asset amounts
        let (pool_asset1_balance, pool_asset2_balance) = get_pool_asset_amounts(
            pool_response,
            &cfg.lp_config.asset_data.asset1,
            &cfg.lp_config.asset_data.asset2,
        )?;

        // Check which asset is being provided and get its balance
        let (mut asset_balance, other_asset) = if asset == cfg.lp_config.asset_data.asset1 {
            (balance_asset1.clone(), balance_asset2.clone())
        } else if asset == cfg.lp_config.asset_data.asset2 {
            (balance_asset2.clone(), balance_asset1.clone())
        } else {
            return Err(LibraryError::ExecutionError(
                "Asset to provide liquidity for is not part of the pool".to_string(),
            ));
        };

        // Check pool ratio if range is provided
        if let Some(range) = expected_pool_ratio_range {
            let pool_asset_ratios =
                cosmwasm_std::Decimal::checked_from_ratio(pool_asset1_balance, pool_asset2_balance)
                    .map_err(|e| LibraryError::ExecutionError(e.to_string()))?;
            range.is_within_range(pool_asset_ratios)?;
        }

        // Check limit if provided
        if let Some(limit) = limit {
            if limit < asset_balance.amount {
                asset_balance.amount = limit;
            }
        }

        // Create liquidity provision message based on pool type
        let messages = match cfg.lp_config.pool_type {
            PoolType::NativeLpToken(_) => astroport_native::create_single_sided_liquidity_msg(
                &deps,
                &cfg,
                &asset_balance,
                &other_asset,
            )?,
            PoolType::Cw20LpToken(_) => astroport_cw20::create_single_sided_liquidity_msg(
                &deps,
                &cfg,
                &asset_balance,
                &other_asset,
            )?,
        };

        let input_account_msgs = execute_on_behalf_of(messages, &cfg.input_addr)?;

        Ok(Response::new()
            .add_message(input_account_msgs)
            .add_attribute("method", "provide_single_sided_liquidity")
            .add_attribute("asset_amount", asset_balance.amount.to_string()))
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
