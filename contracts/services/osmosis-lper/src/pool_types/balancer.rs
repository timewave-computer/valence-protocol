use cosmwasm_std::{
    coin, Coin, CosmosMsg, Decimal, DepsMut, Fraction, Response, StdError, StdResult, Uint128,
};
use osmosis_std::{cosmwasm_to_proto_coins, types::osmosis::gamm::v1beta1::GammQuerier};
use valence_osmosis_utils::utils::{
    get_pool_ratio, get_provide_liquidity_msg, get_provide_ss_liquidity_msg,
    query_pool_asset_balance, query_pool_pm,
};
use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

use crate::valence_service_integration::Config;

pub fn provide_single_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
    asset: String,
    limit: Uint128,
) -> Result<Response, ServiceError> {
    // first we assert the input account balance
    let input_acc_asset_bal = query_pool_asset_balance(&deps, cfg.input_addr.as_str(), &asset)?;

    deps.api.debug(
        format!(
            "input account pool asset balance: {:?}",
            input_acc_asset_bal
        )
        .as_str(),
    );

    let provision_amount = if input_acc_asset_bal.amount > limit {
        limit
    } else {
        input_acc_asset_bal.amount
    };

    let share_out_amt = calculate_share_out_amt_swap(
        &deps,
        cfg.lp_config.pool_id,
        vec![coin(provision_amount.u128(), asset.to_string())],
    )?;

    deps.api
        .debug(&format!("share out amount: {share_out_amt}"));

    let liquidity_provision_msg = get_provide_ss_liquidity_msg(
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_id,
        coin(provision_amount.u128(), asset),
        share_out_amt,
    )?;

    deps.api.debug(&format!(
        "liquidity provision msg: {:?}",
        liquidity_provision_msg
    ));

    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![liquidity_provision_msg], &cfg.input_addr.clone())?;
    deps.api
        .debug(format!("delegated lp msg: {:?}", delegated_input_acc_msgs).as_str());

    Ok(Response::default().add_message(delegated_input_acc_msgs))
}

pub fn provide_double_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
) -> Result<Response, ServiceError> {
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

    let pool_response = query_pool_pm(&deps, cfg.lp_config.pool_id)?;

    let pool_ratio = get_pool_ratio(
        pool_response,
        cfg.lp_config.pool_asset_1.clone(),
        cfg.lp_config.pool_asset_2.clone(),
    )?;

    // TODO: look into SpotPriceRequest to obtain the ratio instead of querying the pool

    let (asset_1_provision_amt, asset_2_provision_amt) =
        calculate_provision_amounts(bal_asset_1.amount, bal_asset_2.amount, pool_ratio)?;

    let provision_coins = vec![
        Coin {
            denom: cfg.lp_config.pool_asset_1.clone(),
            amount: asset_1_provision_amt,
        },
        Coin {
            denom: cfg.lp_config.pool_asset_2.clone(),
            amount: asset_2_provision_amt,
        },
    ];

    let share_out_amt =
        calculate_share_out_amt_no_swap(&deps, cfg.lp_config.pool_id, provision_coins.clone())?;

    let liquidity_provision_msg: CosmosMsg = get_provide_liquidity_msg(
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_id,
        provision_coins,
        share_out_amt,
    )?;

    let delegated_input_acc_msgs =
        execute_on_behalf_of(vec![liquidity_provision_msg], &cfg.input_addr.clone())?;
    deps.api
        .debug(format!("delegated lp msg: {:?}", delegated_input_acc_msgs).as_str());

    Ok(Response::default().add_message(delegated_input_acc_msgs))
}

pub fn calculate_share_out_amt_no_swap(
    deps: &DepsMut,
    pool_id: u64,
    coins_in: Vec<Coin>,
) -> StdResult<String> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    let resp =
        gamm_querier.calc_join_pool_no_swap_shares(pool_id, cosmwasm_to_proto_coins(coins_in))?;
    Ok(resp.shares_out)
}

pub fn calculate_share_out_amt_swap(
    deps: &DepsMut,
    pool_id: u64,
    coin_in: Vec<Coin>,
) -> StdResult<String> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    let resp = gamm_querier.calc_join_pool_shares(pool_id, cosmwasm_to_proto_coins(coin_in))?;

    Ok(resp.share_out_amount)
}

pub fn calculate_provision_amounts(
    asset_1_bal: Uint128,
    asset_2_bal: Uint128,
    pool_ratio: Decimal,
) -> StdResult<(Uint128, Uint128)> {
    // first we assume that we are going to provide all of asset_1 and up to all of asset_2
    // then we get the expected amount of asset_2 tokens to provide
    let expected_asset_2_provision_amt = asset_1_bal
        .checked_multiply_ratio(pool_ratio.numerator(), pool_ratio.denominator())
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // then we check if the expected amount of asset_2 tokens is greater than the available balance
    if expected_asset_2_provision_amt > asset_2_bal {
        // if it is, we calculate the amount of asset_1 tokens to provide
        let asset_1_provision_amt = asset_2_bal
            .checked_multiply_ratio(pool_ratio.denominator(), pool_ratio.numerator())
            .map_err(|e| StdError::generic_err(e.to_string()))?;
        Ok((asset_1_provision_amt, asset_2_bal))
    } else {
        // if it is not, we provide all of asset_1 and the expected amount of asset_2
        Ok((asset_1_bal, expected_asset_2_provision_amt))
    }
}
