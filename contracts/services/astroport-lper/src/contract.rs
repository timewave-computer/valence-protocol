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
    use astroport::{asset::Asset, pair::PoolResponse, DecimalCheckedOps};
    use cosmwasm_std::{
        coin, to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg,
    };
    use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

    use crate::msg::{ActionsMsgs, Config};

    pub fn process_action(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: ActionsMsgs,
        cfg: Config,
    ) -> Result<Response, ServiceError> {
        // Get balances of both assets from input account
        let balance_asset1 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset1)?;
        let balance_asset2 = deps
            .querier
            .query_balance(&cfg.input_addr, &cfg.lp_config.asset_data.asset2)?;

        // Get pool information
        let pool_response: PoolResponse = deps
            .querier
            .query_wasm_smart(&cfg.pool_addr, &astroport::pair::QueryMsg::Pool {})?;

        // Get the amounts of each of the assets of our config in the pool
        let (pool_asset_a_balance, pool_asset_b_balance) = get_pool_asset_amounts(
            pool_response.assets,
            &cfg.lp_config.asset_data.asset1,
            &cfg.lp_config.asset_data.asset2,
        )?;

        // Get the pool asset ratios
        let pool_asset_ratios =
            cosmwasm_std_astroport::Decimal::from_ratio(pool_asset_a_balance, pool_asset_b_balance);

        match msg {
            ActionsMsgs::ProvideDoubleSidedLiquidity {
                expected_pool_ratio_range,
            } => {
                // If we have an expected pool ratio range, we need to check if the pool is within that range
                if let Some(range) = expected_pool_ratio_range {
                    range.is_within_range(pool_asset_ratios)?;
                }

                let required_asset1_amount = pool_asset_ratios
                    .checked_mul_uint128(cosmwasm_std_astroport::Uint128::new(
                        balance_asset2.amount.u128(),
                    ))
                    .map_err(|error| ServiceError::ExecutionError(error.to_string()))?;

                // Let's get the maximum amount of assets that we can provide liquidity
                let (asset1_provide_amount, asset2_provide_amount) =
                    if balance_asset1.amount.u128() >= required_asset1_amount.u128() {
                        // We can provide all asset2 tokens along with the corresponding maximum of asset1 tokens
                        (required_asset1_amount.u128(), balance_asset2.amount.u128())
                    } else {
                        // We can't provide all asset2 tokens so we need to determine how many we can provide according to our available asset1 tokens
                        let ratio = cosmwasm_std_astroport::Decimal::from_ratio(
                            pool_asset_b_balance,
                            pool_asset_a_balance,
                        );

                        (
                            balance_asset1.amount.u128(),
                            ratio
                                .checked_mul_uint128(cosmwasm_std_astroport::Uint128::new(
                                    balance_asset1.amount.u128(),
                                ))
                                .map_err(|error| ServiceError::ExecutionError(error.to_string()))?
                                .u128(),
                        )
                    };

                // Depending on the astroport pool version we are using we will have to use the right structure
                let execute_msg_binary = match cfg.lp_config.pool_type {
                    crate::msg::PoolType::NativeLpToken(_) => {
                        to_json_binary(&astroport::pair::ExecuteMsg::ProvideLiquidity {
                            assets: vec![
                                Asset {
                                    info: astroport::asset::AssetInfo::NativeToken {
                                        denom: cfg.lp_config.asset_data.asset1.to_string(),
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(
                                        asset1_provide_amount,
                                    ),
                                },
                                Asset {
                                    info: astroport::asset::AssetInfo::NativeToken {
                                        denom: cfg.lp_config.asset_data.asset2.to_string(),
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(
                                        asset2_provide_amount,
                                    ),
                                },
                            ],
                            slippage_tolerance: cfg.lp_config.slippage_tolerance,
                            auto_stake: Some(false),
                            receiver: Some(cfg.output_addr.to_string()),
                            min_lp_to_receive: None,
                        })?
                    }
                    crate::msg::PoolType::Cw20LpToken(_) => to_json_binary(
                        &astroport_cw20_lp_token::pair::ExecuteMsg::ProvideLiquidity {
                            assets: vec![
                                astroport_cw20_lp_token::asset::Asset {
                                    info: astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                                        denom: cfg.lp_config.asset_data.asset1.to_string(),
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(
                                        asset1_provide_amount,
                                    ),
                                },
                                astroport_cw20_lp_token::asset::Asset {
                                    info: astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                                        denom: cfg.lp_config.asset_data.asset2.to_string(),
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(
                                        asset2_provide_amount,
                                    ),
                                },
                            ],
                            slippage_tolerance: cfg.lp_config.slippage_tolerance,
                            auto_stake: Some(false),
                            receiver: Some(cfg.output_addr.to_string()),
                        },
                    )?,
                };

                // Create the CosmosMsg that we the input account will execute
                let cosmos_msgs = vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cfg.pool_addr.to_string(),
                    msg: execute_msg_binary,
                    funds: vec![
                        coin(asset1_provide_amount, cfg.lp_config.asset_data.asset1),
                        coin(asset2_provide_amount, cfg.lp_config.asset_data.asset2),
                    ],
                })];

                let input_account_msgs = execute_on_behalf_of(cosmos_msgs, &cfg.input_addr)?;

                Ok(Response::new()
                    .add_message(input_account_msgs)
                    .add_attribute("method", "provide_double_sided_liquidity"))
            }
            ActionsMsgs::ProvideSingleSidedLiquidity {
                asset,
                limit,
                expected_pool_ratio_range,
            } => {
                // We check first that the asset is one of the pool assets and get the balance
                let (asset_balance, other_asset) = if asset == cfg.lp_config.asset_data.asset1 {
                    (balance_asset1.clone(), balance_asset2.clone())
                } else if asset == cfg.lp_config.asset_data.asset2 {
                    (balance_asset2.clone(), balance_asset1.clone())
                } else {
                    return Err(ServiceError::ExecutionError(
                        "Asset to provide liquidity for is not part of the pool".to_string(),
                    ));
                };

                // If we have an expected pool ratio range, we need to check if the pool is within that range
                if let Some(range) = expected_pool_ratio_range {
                    range.is_within_range(pool_asset_ratios)?;
                }

                // If we have a single side limit, check that we are within the limit
                if let Some(limit) = limit {
                    if limit >= asset_balance.amount {
                        return Err(ServiceError::ExecutionError(
                            "Asset amount is greater than the limit".to_string(),
                        ));
                    }
                }

                // We are distinguishing between two different astroport pool versions
                let messages = match cfg.lp_config.pool_type {
                    crate::msg::PoolType::NativeLpToken(pair_type) => match pair_type {
                        // Xyk pools do not allow for automatic single-sided liquidity provision.
                        // We therefore perform a manual swap with 1/2 of the available denom, and execute
                        // a two-sided lp provision with the resulting assets.
                        astroport::factory::PairType::Xyk {} => {
                            let halved_coin = cosmwasm_std_astroport::Coin {
                                denom: asset_balance.denom,
                                amount: cosmwasm_std_astroport::Uint128::from(
                                    asset_balance.amount.u128(),
                                ) / cosmwasm_std_astroport::Uint128::from(2u128),
                            };

                            let (offer_asset, mut ask_asset) = {
                                if balance_asset1.denom.clone() == halved_coin.denom {
                                    (
                                        cosmwasm_std_astroport::coin(
                                            halved_coin.amount.u128(),
                                            balance_asset1.denom,
                                        ),
                                        cosmwasm_std_astroport::coin(
                                            balance_asset2.amount.u128(),
                                            balance_asset2.denom,
                                        ),
                                    )
                                } else {
                                    (
                                        cosmwasm_std_astroport::coin(
                                            halved_coin.amount.u128(),
                                            balance_asset2.denom,
                                        ),
                                        cosmwasm_std_astroport::coin(
                                            balance_asset1.amount.u128(),
                                            balance_asset1.denom,
                                        ),
                                    )
                                }
                            };

                            let astroport_offer_asset = Asset {
                                info: astroport::asset::AssetInfo::NativeToken {
                                    denom: offer_asset.denom.clone(),
                                },
                                amount: cosmwasm_std_astroport::Uint128::new(
                                    offer_asset.amount.u128(),
                                ),
                            };

                            let simulation: astroport::pair::SimulationResponse =
                                deps.querier.query_wasm_smart(
                                    &cfg.pool_addr,
                                    &astroport::pair::QueryMsg::Simulation {
                                        offer_asset: astroport_offer_asset.clone(),
                                        ask_asset_info: None,
                                    },
                                )?;

                            ask_asset.amount = simulation.return_amount;

                            let swap_wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.pool_addr.to_string(),
                                msg: to_json_binary(&astroport::pair::ExecuteMsg::Swap {
                                    offer_asset: astroport_offer_asset.clone(),
                                    max_spread: cfg.lp_config.slippage_tolerance,
                                    belief_price: None,
                                    to: None,
                                    ask_asset_info: None,
                                })?,
                                funds: vec![coin(ask_asset.amount.u128(), ask_asset.denom.clone())],
                            });

                            let providy_liquidity_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.pool_addr.to_string(),
                                msg: to_json_binary(
                                    &astroport::pair::ExecuteMsg::ProvideLiquidity {
                                        assets: vec![
                                            astroport_offer_asset,
                                            Asset {
                                                info: astroport::asset::AssetInfo::NativeToken {
                                                    denom: ask_asset.denom.clone(),
                                                },
                                                amount: cosmwasm_std_astroport::Uint128::new(
                                                    ask_asset.amount.u128(),
                                                ),
                                            },
                                        ],
                                        slippage_tolerance: cfg.lp_config.slippage_tolerance,
                                        auto_stake: Some(false),
                                        receiver: Some(cfg.output_addr.to_string()),
                                        min_lp_to_receive: None,
                                    },
                                )?,
                                funds: vec![
                                    coin(offer_asset.amount.u128(), offer_asset.denom),
                                    coin(ask_asset.amount.u128(), ask_asset.denom),
                                ],
                            });
                            vec![swap_wasm_msg, providy_liquidity_msg]
                        }
                        astroport::factory::PairType::Stable {}
                        | astroport::factory::PairType::Custom(_) => {
                            // Provide the liquidity with only one non-zero asset
                            let assets = vec![
                                Asset {
                                    info: astroport::asset::AssetInfo::NativeToken {
                                        denom: asset_balance.denom.clone(),
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(
                                        asset_balance.amount.u128(),
                                    ),
                                },
                                Asset {
                                    info: astroport::asset::AssetInfo::NativeToken {
                                        denom: other_asset.denom,
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(0),
                                },
                            ];

                            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.pool_addr.to_string(),
                                msg: to_json_binary(
                                    &astroport::pair::ExecuteMsg::ProvideLiquidity {
                                        assets,
                                        slippage_tolerance: cfg.lp_config.slippage_tolerance,
                                        auto_stake: Some(false),
                                        receiver: Some(cfg.output_addr.to_string()),
                                        min_lp_to_receive: None,
                                    },
                                )?,
                                funds: vec![coin(asset_balance.amount.u128(), asset_balance.denom)],
                            })]
                        }
                    },
                    crate::msg::PoolType::Cw20LpToken(pair_type) => match pair_type {
                        astroport_cw20_lp_token::factory::PairType::Xyk {} => {
                            let halved_coin = cosmwasm_std_astroport::Coin {
                                denom: asset_balance.denom,
                                amount: cosmwasm_std_astroport::Uint128::from(
                                    asset_balance.amount.u128(),
                                ) / cosmwasm_std_astroport::Uint128::from(2u128),
                            };

                            let (offer_asset, mut ask_asset) = {
                                if balance_asset1.denom.clone() == halved_coin.denom {
                                    (
                                        cosmwasm_std_astroport::coin(
                                            halved_coin.amount.u128(),
                                            balance_asset1.denom,
                                        ),
                                        cosmwasm_std_astroport::coin(
                                            balance_asset2.amount.u128(),
                                            balance_asset2.denom,
                                        ),
                                    )
                                } else {
                                    (
                                        cosmwasm_std_astroport::coin(
                                            halved_coin.amount.u128(),
                                            balance_asset2.denom,
                                        ),
                                        cosmwasm_std_astroport::coin(
                                            balance_asset1.amount.u128(),
                                            balance_asset1.denom,
                                        ),
                                    )
                                }
                            };

                            let astroport_offer_asset = astroport_cw20_lp_token::asset::Asset {
                                info: astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                                    denom: offer_asset.denom.clone(),
                                },
                                amount: cosmwasm_std_astroport::Uint128::new(
                                    offer_asset.amount.u128(),
                                ),
                            };

                            let simulation: astroport_cw20_lp_token::pair::SimulationResponse =
                                deps.querier.query_wasm_smart(
                                    &cfg.pool_addr,
                                    &astroport_cw20_lp_token::pair::QueryMsg::Simulation {
                                        offer_asset: astroport_offer_asset.clone(),
                                        ask_asset_info: None,
                                    },
                                )?;

                            ask_asset.amount = simulation.return_amount;

                            let swap_wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.pool_addr.to_string(),
                                msg: to_json_binary(
                                    &astroport_cw20_lp_token::pair::ExecuteMsg::Swap {
                                        offer_asset: astroport_offer_asset.clone(),
                                        max_spread: cfg.lp_config.slippage_tolerance,
                                        belief_price: None,
                                        to: None,
                                        ask_asset_info: None,
                                    },
                                )?,
                                funds: vec![coin(ask_asset.amount.u128(), ask_asset.denom.clone())],
                            });

                            let providy_liquidity_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.pool_addr.to_string(),
                                msg: to_json_binary(
                                    &astroport_cw20_lp_token::pair::ExecuteMsg::ProvideLiquidity {
                                        assets: vec![
                                            astroport_offer_asset,
                                            astroport_cw20_lp_token::asset::Asset {
                                                info: astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                                                    denom: ask_asset.denom.clone(),
                                                },
                                                amount: cosmwasm_std_astroport::Uint128::new(
                                                    ask_asset.amount.u128(),
                                                ),
                                            },
                                        ],
                                        slippage_tolerance: cfg.lp_config.slippage_tolerance,
                                        auto_stake: Some(false),
                                        receiver: Some(cfg.output_addr.to_string()),
                                    },
                                )?,
                                funds: vec![
                                    coin(offer_asset.amount.u128(), offer_asset.denom),
                                    coin(ask_asset.amount.u128(), ask_asset.denom),
                                ],
                            });

                            vec![swap_wasm_msg, providy_liquidity_msg]
                        }

                        astroport_cw20_lp_token::factory::PairType::Stable {}
                        | astroport_cw20_lp_token::factory::PairType::Custom(_) => {
                            // Provide the liquidity with only one non-zero asset
                            let assets = vec![
                                astroport_cw20_lp_token::asset::Asset {
                                    info: astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                                        denom: asset_balance.denom.clone(),
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(
                                        asset_balance.amount.u128(),
                                    ),
                                },
                                astroport_cw20_lp_token::asset::Asset {
                                    info: astroport_cw20_lp_token::asset::AssetInfo::NativeToken {
                                        denom: other_asset.denom,
                                    },
                                    amount: cosmwasm_std_astroport::Uint128::new(0),
                                },
                            ];

                            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: cfg.pool_addr.to_string(),
                                msg: to_json_binary(
                                    &astroport_cw20_lp_token::pair::ExecuteMsg::ProvideLiquidity {
                                        assets,
                                        slippage_tolerance: cfg.lp_config.slippage_tolerance,
                                        auto_stake: Some(false),
                                        receiver: Some(cfg.output_addr.to_string()),
                                    },
                                )?,
                                funds: vec![coin(asset_balance.amount.u128(), asset_balance.denom)],
                            })]
                        }
                    },
                };

                Ok(Response::new()
                    .add_messages(messages)
                    .add_attribute("method", "provide_single_sided_liquidity"))
            }
        }
    }

    fn get_pool_asset_amounts(
        assets: Vec<Asset>,
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
