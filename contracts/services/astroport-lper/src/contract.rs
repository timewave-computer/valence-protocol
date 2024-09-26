#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::{
    error::{ServiceError, UnauthorizedReason},
    msg::{Config, ExecuteMsg, InstantiateMsg, QueryMsg, ServiceConfigValidation},
    state::{CONFIG, PROCESSOR},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ServiceError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    PROCESSOR.save(deps.storage, &deps.api.addr_validate(&msg.processor)?)?;

    let config = msg.config.validate(deps.as_ref())?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ServiceError> {
    match msg {
        ExecuteMsg::ProcessAction(action_msgs) => {
            let processor = PROCESSOR.load(deps.storage)?;
            if info.sender != processor {
                return Err(ServiceError::Unauthorized(
                    UnauthorizedReason::NotAllowed {},
                ));
            }
            let config = CONFIG.load(deps.storage)?;
            actions::process_action(deps, env, info, action_msgs, config)
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            cw_ownable::assert_owner(deps.as_ref().storage, &info.sender)?;
            let config = new_config.validate(deps.as_ref())?;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::new().add_attribute("method", "update_config"))
        }
        ExecuteMsg::UpdateProcessor { processor } => {
            cw_ownable::assert_owner(deps.as_ref().storage, &info.sender)?;
            PROCESSOR.save(deps.storage, &deps.api.addr_validate(&processor)?)?;
            Ok(Response::default()
                .add_attribute("method", "update_processor")
                .add_attribute("processor", processor))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let result =
                cw_ownable::update_ownership(deps, &env.block, &info.sender, action.clone())?;
            Ok(Response::default()
                .add_attribute("method", "update_ownership")
                .add_attribute("action", format!("{:?}", action))
                .add_attribute("result", format!("{:?}", result)))
        }
    }
}

mod actions {
    use astroport::DecimalCheckedOps;
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{
        to_json_binary, Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult,
        Uint128, WasmMsg,
    };

    use crate::{
        astroport_cw20, astroport_native,
        error::ServiceError,
        msg::{ActionsMsgs, Config, DecimalRange, PoolType},
    };

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        match msg {
            ActionsMsgs::ProvideDoubleSidedLiquidity {
                expected_pool_ratio_range,
            } => provide_double_sided_liquidity(deps, cfg, expected_pool_ratio_range),
            ActionsMsgs::ProvideSingleSidedLiquidity {
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
    ) -> Result<Response, ServiceError> {
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
            cosmwasm_std::Decimal::from_ratio(pool_asset1_balance, pool_asset2_balance);

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

    fn query_asset_balances(deps: &DepsMut, cfg: &Config) -> Result<(Coin, Coin), ServiceError> {
        let balance_asset1 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset1)?;
        let balance_asset2 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset2)?;
        Ok((balance_asset1, balance_asset2))
    }

    fn query_pool(
        deps: &DepsMut,
        pool_addr: &str,
        pool_type: &PoolType,
    ) -> Result<Vec<Box<dyn AssetTrait>>, ServiceError> {
        match pool_type {
            PoolType::NativeLpToken(_) => {
                let assets = astroport_native::query_pool(deps, pool_addr)?;
                Ok(assets
                    .into_iter()
                    .map(|asset| Box::new(asset) as Box<dyn AssetTrait>)
                    .collect())
            }
            PoolType::Cw20LpToken(_) => {
                let assets = astroport_cw20::query_pool(deps, pool_addr)?;
                Ok(assets
                    .into_iter()
                    .map(|asset| Box::new(asset) as Box<dyn AssetTrait>)
                    .collect())
            }
        }
    }

    fn get_pool_asset_amounts(
        assets: Vec<Box<dyn AssetTrait>>,
        asset1_denom: &str,
        asset2_denom: &str,
    ) -> Result<(u128, u128), ServiceError> {
        let (mut asset1_balance, mut asset2_balance) = (0, 0);

        for asset in assets {
            let coin = asset
                .as_coin()
                .map_err(|error| ServiceError::ExecutionError(error.to_string()))?;

            if coin.denom == asset1_denom {
                asset1_balance = coin.amount.u128();
            } else if coin.denom == asset2_denom {
                asset2_balance = coin.amount.u128();
            }
        }

        if asset1_balance == 0 || asset2_balance == 0 {
            return Err(ServiceError::ExecutionError(
                "All pool assets must be non-zero".to_string(),
            ));
        }

        Ok((asset1_balance, asset2_balance))
    }

    fn calculate_provide_amounts(
        balance1: u128,
        balance2: u128,
        pool_asset1_balance: u128,
        pool_asset2_balance: u128,
        pool_asset_ratio: cosmwasm_std::Decimal,
    ) -> Result<(u128, u128), ServiceError> {
        // Let's get the maximum amount of assets that we can provide liquidity
        let required_asset1_amount = pool_asset_ratio
            .checked_mul_uint128(cosmwasm_std::Uint128::from(balance2))
            .map_err(|error| ServiceError::ExecutionError(error.to_string()))?;

        // We can provide all asset2 tokens along with the corresponding maximum of asset1 tokens
        if balance1 >= required_asset1_amount.u128() {
            Ok((required_asset1_amount.u128(), balance2))
        } else {
            // We can't provide all asset2 tokens so we need to determine how many we can provide according to our available asset1
            let ratio = cosmwasm_std::Decimal::from_ratio(pool_asset1_balance, pool_asset2_balance);

            Ok((
                balance1,
                ratio
                    .checked_mul_uint128(cosmwasm_std::Uint128::new(balance1))
                    .map_err(|error| ServiceError::ExecutionError(error.to_string()))?
                    .u128(),
            ))
        }
    }

    fn create_provide_liquidity_msg(
        cfg: &Config,
        amount1: u128,
        amount2: u128,
    ) -> Result<CosmosMsg, ServiceError> {
        match &cfg.lp_config.pool_type {
            PoolType::NativeLpToken(_) => {
                astroport_native::create_provide_liquidity_msg(cfg, amount1, amount2)
            }
            PoolType::Cw20LpToken(_) => {
                astroport_cw20::create_provide_liquidity_msg(cfg, amount1, amount2)
            }
        }
    }

    // Define a trait that both Asset types can implement
    pub trait AssetTrait {
        fn as_coin(&self) -> Result<cosmwasm_std::Coin, ServiceError>;
    }

    // Implement the trait for both Asset types
    impl AssetTrait for astroport::asset::Asset {
        fn as_coin(&self) -> Result<cosmwasm_std::Coin, ServiceError> {
            self.as_coin()
                .map_err(|error| ServiceError::ExecutionError(error.to_string()))
        }
    }

    impl AssetTrait for astroport_cw20_lp_token::asset::Asset {
        fn as_coin(&self) -> Result<cosmwasm_std::Coin, ServiceError> {
            self.to_coin()
                .map_err(|error| ServiceError::ExecutionError(error.to_string()))
        }
    }

    fn provide_single_sided_liquidity(
        deps: DepsMut,
        cfg: Config,
        asset: String,
        limit: Option<Uint128>,
        expected_pool_ratio_range: Option<DecimalRange>,
    ) -> Result<Response, ServiceError> {
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
        let (asset_balance, other_asset) = if asset == cfg.lp_config.asset_data.asset1 {
            (balance_asset1.clone(), balance_asset2.clone())
        } else if asset == cfg.lp_config.asset_data.asset2 {
            (balance_asset2.clone(), balance_asset1.clone())
        } else {
            return Err(ServiceError::ExecutionError(
                "Asset to provide liquidity for is not part of the pool".to_string(),
            ));
        };

        // Check pool ratio if range is provided
        if let Some(range) = expected_pool_ratio_range {
            let pool_asset_ratios =
                cosmwasm_std::Decimal::from_ratio(pool_asset1_balance, pool_asset2_balance);
            range.is_within_range(pool_asset_ratios)?;
        }

        // Check limit if provided
        if let Some(limit) = limit {
            if limit < asset_balance.amount {
                return Err(ServiceError::ExecutionError(
                    "Asset amount is greater than the limit".to_string(),
                ));
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

    // This is a helper function to execute a CosmosMsg on behalf of an account
    pub fn execute_on_behalf_of(msgs: Vec<CosmosMsg>, account: &Addr) -> StdResult<CosmosMsg> {
        // Used to execute a CosmosMsg on behalf of an account
        #[cw_serde]
        pub enum ExecuteMsg {
            ExecuteMsg { msgs: Vec<CosmosMsg> }, // Execute any CosmosMsg (approved services or admin)
        }

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: account.to_string(),
            msg: to_json_binary(&ExecuteMsg::ExecuteMsg { msgs })?,
            funds: vec![],
        }))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::GetProcessor {} => {
            let processor = PROCESSOR.load(deps.storage)?;
            to_json_binary(&processor)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = CONFIG.load(deps.storage)?;
            to_json_binary(&config)
        }
    }
}
