use cosmwasm_std::{Coin, DepsMut, Response, StdResult};
use osmosis_std::cosmwasm_to_proto_coins;
use valence_osmosis_utils::utils::{query_pool_asset_balance, query_pool_pm};
use valence_service_utils::error::ServiceError;

use crate::valence_service_integration::Config;

pub fn provide_double_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
) -> Result<Response, ServiceError> {
    deps.api
        .debug("provide double sided liquidity for cosmwasm liquidity pool");

    // first we assert the input account balances
    let bal_asset_1 = query_pool_asset_balance(
        &deps,
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_asset_1.as_str(),
    )?;
    let bal_asset_2 = query_pool_asset_balance(
        &deps,
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_asset_2.as_str(),
    )?;

    deps.api
        .debug(format!("input account pool asset 1 balance: {:?}", bal_asset_1).as_str());
    deps.api
        .debug(format!("input account pool asset 2 balance: {:?}", bal_asset_2).as_str());

    let cw_querier =
        osmosis_std::types::osmosis::cosmwasmpool::v1beta1::CosmwasmpoolQuerier::new(&deps.querier);
    let contract_info = cw_querier.contract_info_by_pool_id(cfg.lp_config.pool_id)?;
    deps.api
        .debug(format!("cw pool contract info response: {:?}", contract_info).as_str());
    let pm_querier =
        osmosis_std::types::osmosis::poolmanager::v2::PoolmanagerQuerier::new(&deps.querier);
    let spot_price_query = pm_querier.spot_price_v2(
        cfg.lp_config.pool_id,
        cfg.lp_config.pool_asset_1,
        cfg.lp_config.pool_asset_2,
    )?;

    deps.api
        .debug(format!("spot price query response: {:?}", spot_price_query).as_str());

    Ok(Response::default())
}

// pub fn calculate_share_out_amt_no_swap(
//     deps: &DepsMut,
//     pool_id: u64,
//     coins_in: Vec<Coin>,
// ) -> StdResult<String> {
//     let pm_querier =
//         osmosis_std::types::osmosis::poolmanager::v1beta1::PoolmanagerQuerier::new(&deps.querier);
//     let pm_querier_v2 =
//         osmosis_std::types::osmosis::poolmanager::v2::PoolmanagerQuerier::new(&deps.querier);

//     let proto_coins_in = cosmwasm_to_proto_coins(coins_in);
//     let resp = pm_querier.estimate_single_pool_swap_exact_amount_in(pool_id)?;
//     Ok(resp.shares_out)
// }

// pub fn calculate_share_out_amt_swap(
//     deps: &DepsMut,
//     pool_id: u64,
//     coin_in: Vec<Coin>,
// ) -> StdResult<String> {
//     let gamm_querier = GammQuerier::new(&deps.querier);
//     let resp = gamm_querier.calc_join_pool_shares(pool_id, cosmwasm_to_proto_coins(coin_in))?;

//     Ok(resp.share_out_amount)
// }
