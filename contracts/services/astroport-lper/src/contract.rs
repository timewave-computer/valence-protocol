#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use valence_service_utils::{
    error::ServiceError,
    msg::{ExecuteMsg, InstantiateMsg},
};

use crate::msg::{ActionsMsgs, Config, OptionalServiceConfig, QueryMsg, ServiceConfig};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<ServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::instantiate(deps, CONTRACT_NAME, CONTRACT_VERSION, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<ActionsMsgs, OptionalServiceConfig>,
) -> Result<Response, ServiceError> {
    valence_service_base::execute(
        deps,
        env,
        info,
        msg,
        actions::process_action,
        execute::update_config,
    )
}

mod actions {
    use astroport::DecimalCheckedOps;
    use cosmwasm_std::{Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::msg::{ActionsMsgs, Config, DecimalRange, PoolType};

    use super::{astroport_cw20, astroport_native};

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
            cosmwasm_std_astroport::Decimal::from_ratio(pool_asset1_balance, pool_asset2_balance);

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
        pool_asset_ratio: cosmwasm_std_astroport::Decimal,
    ) -> Result<(u128, u128), ServiceError> {
        // Let's get the maximum amount of assets that we can provide liquidity
        let required_asset1_amount = pool_asset_ratio
            .checked_mul_uint128(cosmwasm_std_astroport::Uint128::from(balance2))
            .map_err(|error| ServiceError::ExecutionError(error.to_string()))?;

        // We can provide all asset2 tokens along with the corresponding maximum of asset1 tokens
        if balance1 >= required_asset1_amount.u128() {
            Ok((required_asset1_amount.u128(), balance2))
        } else {
            // We can't provide all asset2 tokens so we need to determine how many we can provide according to our available asset1
            let ratio = cosmwasm_std_astroport::Decimal::from_ratio(
                pool_asset1_balance,
                pool_asset2_balance,
            );

            Ok((
                balance1,
                ratio
                    .checked_mul_uint128(cosmwasm_std_astroport::Uint128::new(balance1))
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
        fn as_coin(&self) -> Result<cosmwasm_std_astroport::Coin, ServiceError>;
    }

    // Implement the trait for both Asset types
    impl AssetTrait for astroport::asset::Asset {
        fn as_coin(&self) -> Result<cosmwasm_std_astroport::Coin, ServiceError> {
            self.as_coin()
                .map_err(|error| ServiceError::ExecutionError(error.to_string()))
        }
    }

    impl AssetTrait for astroport_cw20_lp_token::asset::Asset {
        fn as_coin(&self) -> Result<cosmwasm_std_astroport::Coin, ServiceError> {
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
            let pool_asset_ratios = cosmwasm_std_astroport::Decimal::from_ratio(
                pool_asset1_balance,
                pool_asset2_balance,
            );
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
}

mod astroport_native {
    use crate::msg::PoolType;

    use super::*;
    use astroport::asset::{Asset, AssetInfo};
    use astroport::pair::{ExecuteMsg, PoolResponse, QueryMsg};
    use cosmwasm_std::{coin, CosmosMsg, WasmMsg};
    use cosmwasm_std_astroport::Uint128;

    pub fn query_pool(deps: &DepsMut, pool_addr: &str) -> Result<Vec<Asset>, ServiceError> {
        let response: PoolResponse = deps
            .querier
            .query_wasm_smart(pool_addr, &QueryMsg::Pool {})?;
        Ok(response.assets)
    }

    pub fn create_provide_liquidity_msg(
        cfg: &Config,
        amount1: u128,
        amount2: u128,
    ) -> Result<CosmosMsg, ServiceError> {
        let execute_msg = ExecuteMsg::ProvideLiquidity {
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: cfg.lp_config.asset_data.asset1.to_string(),
                    },
                    amount: Uint128::new(amount1),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: cfg.lp_config.asset_data.asset2.to_string(),
                    },
                    amount: Uint128::new(amount2),
                },
            ],
            slippage_tolerance: cfg.lp_config.slippage_tolerance,
            auto_stake: Some(false),
            receiver: Some(cfg.output_addr.to_string()),
            min_lp_to_receive: None,
        };

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&execute_msg)?,
            funds: vec![
                coin(amount1, &cfg.lp_config.asset_data.asset1),
                coin(amount2, &cfg.lp_config.asset_data.asset2),
            ],
        }))
    }

    pub fn create_single_sided_liquidity_msg(
        deps: &DepsMut,
        cfg: &Config,
        asset_balance: &cosmwasm_std::Coin,
        other_asset: &cosmwasm_std::Coin,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        match cfg.lp_config.pool_type.clone() {
            PoolType::NativeLpToken(pair_type) => match pair_type {
                astroport::factory::PairType::Xyk {} => {
                    create_xyk_liquidity_msg(deps, cfg, asset_balance, other_asset)
                }
                astroport::factory::PairType::Stable {}
                | astroport::factory::PairType::Custom(_) => {
                    create_stable_or_custom_liquidity_msg(cfg, asset_balance, other_asset)
                }
            },
            _ => Err(ServiceError::ExecutionError(
                "Invalid pool type for astroport_native".to_string(),
            )),
        }
    }

    fn create_xyk_liquidity_msg(
        deps: &DepsMut,
        cfg: &Config,
        asset_balance: &cosmwasm_std::Coin,
        other_asset: &cosmwasm_std::Coin,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        let halved_coin = cosmwasm_std_astroport::Coin {
            denom: asset_balance.denom.clone(),
            amount: cosmwasm_std_astroport::Uint128::from(asset_balance.amount.u128())
                / cosmwasm_std_astroport::Uint128::from(2u128),
        };

        let (offer_asset, mut ask_asset) = {
            (
                cosmwasm_std_astroport::coin(
                    halved_coin.amount.u128(),
                    asset_balance.denom.clone(),
                ),
                cosmwasm_std_astroport::coin(other_asset.amount.u128(), other_asset.denom.clone()),
            )
        };

        let astroport_offer_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: offer_asset.denom.clone(),
            },
            amount: Uint128::new(offer_asset.amount.u128()),
        };

        // Simulate swap
        let simulation: astroport::pair::SimulationResponse = deps.querier.query_wasm_smart(
            &cfg.pool_addr,
            &QueryMsg::Simulation {
                offer_asset: astroport_offer_asset.clone(),
                ask_asset_info: None,
            },
        )?;

        ask_asset.amount = simulation.return_amount;

        let swap_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::Swap {
                offer_asset: astroport_offer_asset.clone(),
                max_spread: cfg.lp_config.slippage_tolerance,
                belief_price: None,
                to: None,
                ask_asset_info: None,
            })?,
            funds: vec![coin(ask_asset.amount.u128(), ask_asset.denom.clone())],
        });

        let provide_liquidity_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::ProvideLiquidity {
                assets: vec![
                    astroport_offer_asset,
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ask_asset.denom.clone(),
                        },
                        amount: Uint128::new(ask_asset.amount.u128()),
                    },
                ],
                slippage_tolerance: cfg.lp_config.slippage_tolerance,
                auto_stake: Some(false),
                receiver: Some(cfg.output_addr.to_string()),
                min_lp_to_receive: None,
            })?,
            funds: vec![
                coin(offer_asset.amount.u128(), offer_asset.denom),
                coin(ask_asset.amount.u128(), ask_asset.denom),
            ],
        });

        Ok(vec![swap_msg, provide_liquidity_msg])
    }

    fn create_stable_or_custom_liquidity_msg(
        cfg: &Config,
        asset_balance: &cosmwasm_std::Coin,
        other_asset: &cosmwasm_std::Coin,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        let assets = vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: asset_balance.denom.clone(),
                },
                amount: Uint128::new(asset_balance.amount.u128()),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: other_asset.denom.clone(),
                },
                amount: Uint128::new(0),
            },
        ];

        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::ProvideLiquidity {
                assets,
                slippage_tolerance: cfg.lp_config.slippage_tolerance,
                auto_stake: Some(false),
                receiver: Some(cfg.output_addr.to_string()),
                min_lp_to_receive: None,
            })?,
            funds: vec![coin(
                asset_balance.amount.u128(),
                asset_balance.denom.clone(),
            )],
        });

        Ok(vec![msg])
    }
}

mod astroport_cw20 {
    use crate::msg::PoolType;

    use super::*;
    use astroport_cw20_lp_token::asset::{Asset, AssetInfo};
    use astroport_cw20_lp_token::pair::{ExecuteMsg, PoolResponse, QueryMsg};
    use cosmwasm_std::{coin, CosmosMsg, WasmMsg};
    use cosmwasm_std_astroport::Uint128;

    pub fn query_pool(deps: &DepsMut, pool_addr: &str) -> Result<Vec<Asset>, ServiceError> {
        let response: PoolResponse = deps
            .querier
            .query_wasm_smart(pool_addr, &QueryMsg::Pool {})?;
        Ok(response.assets)
    }

    pub fn create_provide_liquidity_msg(
        cfg: &Config,
        amount1: u128,
        amount2: u128,
    ) -> Result<CosmosMsg, ServiceError> {
        let execute_msg = ExecuteMsg::ProvideLiquidity {
            assets: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: cfg.lp_config.asset_data.asset1.to_string(),
                    },
                    amount: Uint128::new(amount1),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: cfg.lp_config.asset_data.asset2.to_string(),
                    },
                    amount: Uint128::new(amount2),
                },
            ],
            slippage_tolerance: cfg.lp_config.slippage_tolerance,
            auto_stake: Some(false),
            receiver: Some(cfg.output_addr.to_string()),
        };

        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&execute_msg)?,
            funds: vec![
                coin(amount1, &cfg.lp_config.asset_data.asset1),
                coin(amount2, &cfg.lp_config.asset_data.asset2),
            ],
        }))
    }

    pub fn create_single_sided_liquidity_msg(
        deps: &DepsMut,
        cfg: &Config,
        asset_balance: &cosmwasm_std::Coin,
        other_asset: &cosmwasm_std::Coin,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        match cfg.lp_config.pool_type.clone() {
            PoolType::Cw20LpToken(pair_type) => match pair_type {
                astroport_cw20_lp_token::factory::PairType::Xyk {} => {
                    create_xyk_liquidity_msg(deps, cfg, asset_balance, other_asset)
                }
                astroport_cw20_lp_token::factory::PairType::Stable {}
                | astroport_cw20_lp_token::factory::PairType::Custom(_) => {
                    create_stable_or_custom_liquidity_msg(cfg, asset_balance, other_asset)
                }
            },
            _ => Err(ServiceError::ExecutionError(
                "Invalid pool type for astroport_cw20".to_string(),
            )),
        }
    }

    fn create_xyk_liquidity_msg(
        deps: &DepsMut,
        cfg: &Config,
        asset_balance: &cosmwasm_std::Coin,
        other_asset: &cosmwasm_std::Coin,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        let halved_coin = cosmwasm_std_astroport::Coin {
            denom: asset_balance.denom.clone(),
            amount: cosmwasm_std_astroport::Uint128::from(asset_balance.amount.u128())
                / cosmwasm_std_astroport::Uint128::from(2u128),
        };

        let (offer_asset, mut ask_asset) = {
            (
                cosmwasm_std_astroport::coin(
                    halved_coin.amount.u128(),
                    asset_balance.denom.clone(),
                ),
                cosmwasm_std_astroport::coin(other_asset.amount.u128(), other_asset.denom.clone()),
            )
        };

        let astroport_offer_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: offer_asset.denom.clone(),
            },
            amount: Uint128::new(offer_asset.amount.u128()),
        };

        // Simulate swap
        let simulation: astroport_cw20_lp_token::pair::SimulationResponse =
            deps.querier.query_wasm_smart(
                &cfg.pool_addr,
                &QueryMsg::Simulation {
                    offer_asset: astroport_offer_asset.clone(),
                    ask_asset_info: None,
                },
            )?;

        ask_asset.amount = simulation.return_amount;

        let swap_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::Swap {
                offer_asset: astroport_offer_asset.clone(),
                max_spread: cfg.lp_config.slippage_tolerance,
                belief_price: None,
                to: None,
                ask_asset_info: None,
            })?,
            funds: vec![coin(ask_asset.amount.u128(), ask_asset.denom.clone())],
        });

        let provide_liquidity_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::ProvideLiquidity {
                assets: vec![
                    astroport_offer_asset,
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: ask_asset.denom.clone(),
                        },
                        amount: Uint128::new(ask_asset.amount.u128()),
                    },
                ],
                slippage_tolerance: cfg.lp_config.slippage_tolerance,
                auto_stake: Some(false),
                receiver: Some(cfg.output_addr.to_string()),
            })?,
            funds: vec![
                coin(offer_asset.amount.u128(), offer_asset.denom),
                coin(ask_asset.amount.u128(), ask_asset.denom),
            ],
        });

        Ok(vec![swap_msg, provide_liquidity_msg])
    }

    fn create_stable_or_custom_liquidity_msg(
        cfg: &Config,
        asset_balance: &cosmwasm_std::Coin,
        other_asset: &cosmwasm_std::Coin,
    ) -> Result<Vec<CosmosMsg>, ServiceError> {
        let assets = vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: asset_balance.denom.clone(),
                },
                amount: Uint128::new(asset_balance.amount.u128()),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: other_asset.denom.clone(),
                },
                amount: Uint128::new(0),
            },
        ];

        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.pool_addr.to_string(),
            msg: to_json_binary(&ExecuteMsg::ProvideLiquidity {
                assets,
                slippage_tolerance: cfg.lp_config.slippage_tolerance,
                auto_stake: Some(false),
                receiver: Some(cfg.output_addr.to_string()),
            })?,
            funds: vec![coin(
                asset_balance.amount.u128(),
                asset_balance.denom.clone(),
            )],
        });

        Ok(vec![msg])
    }
}

mod execute {
    use cosmwasm_std::{DepsMut, Env, MessageInfo};
    use valence_service_utils::error::ServiceError;

    use crate::msg::{Config, OptionalServiceConfig};

    pub fn update_config(
        deps: &DepsMut,
        _env: Env,
        _info: MessageInfo,
        config: &mut Config,
        new_config: OptionalServiceConfig,
    ) -> Result<(), ServiceError> {
        new_config.update_config(deps, config)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => {
            to_json_binary(&valence_service_base::get_ownership(deps.storage)?)
        }
        QueryMsg::GetProcessor {} => {
            to_json_binary(&valence_service_base::get_processor(deps.storage)?)
        }
        QueryMsg::GetServiceConfig {} => {
            let config: Config = valence_service_base::load_config(deps.storage)?;
            to_json_binary(&config)
        }
    }
}
