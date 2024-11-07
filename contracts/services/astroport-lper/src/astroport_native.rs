use crate::msg::{Config, PoolType};
use cosmwasm_std::{coin, CosmosMsg, WasmMsg};
use cosmwasm_std::{to_json_binary, DepsMut, Uint128};
use valence_astroport_utils::astroport_native_lp_token::{
    Asset, AssetInfo, ExecuteMsg, PairType, PoolQueryMsg, PoolResponse, SimulationResponse,
};
use valence_service_utils::error::ServiceError;

pub fn query_pool(deps: &DepsMut, pool_addr: &str) -> Result<Vec<Asset>, ServiceError> {
    let response: PoolResponse = deps
        .querier
        .query_wasm_smart(pool_addr, &PoolQueryMsg::Pool {})?;
    Ok(response.assets)
}

/// Creates a provide liquidity message for an astroport pool that will mint LP tokenfactory tokens
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
            PairType::Xyk {} => create_xyk_liquidity_msg(deps, cfg, asset_balance, other_asset),
            PairType::Stable {} | PairType::Custom(_) => {
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
    // Xyk pools do not allow for automatic single-sided liquidity provision.
    // We therefore perform a manual swap with 1/2 of the available denom, and execute
    // two-sided lp provision with the resulting assets.

    // We halve the non-zero coin we have in order to swap it for the other denom.
    // The halved coin amount here is the floor of the division result,
    // so it is safe to assume that after the swap we will have at least
    // the same amount of the offer asset left.
    let halved_coin = cosmwasm_std::Coin {
        denom: asset_balance.denom.clone(),
        amount: cosmwasm_std::Uint128::from(asset_balance.amount.u128())
            .checked_div(cosmwasm_std::Uint128::from(2u128))
            .expect("denominator is not zero; qed"),
    };

    let (offer_asset, mut ask_asset) = {
        (
            cosmwasm_std::coin(halved_coin.amount.u128(), asset_balance.denom.clone()),
            cosmwasm_std::coin(other_asset.amount.u128(), other_asset.denom.clone()),
        )
    };

    let astroport_offer_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: offer_asset.denom.clone(),
        },
        amount: Uint128::new(offer_asset.amount.u128()),
    };

    // We simulate a swap with 1/2 of the offer asset
    let simulation: SimulationResponse = deps.querier.query_wasm_smart(
        &cfg.pool_addr,
        &PoolQueryMsg::Simulation {
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
        funds: vec![coin(offer_asset.amount.u128(), offer_asset.denom.clone())],
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

    // Given one non-zero asset, we build the ProvideLiquidity message
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
