use std::str::FromStr;

use cosmwasm_std::{
    coin, coins, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Empty, Fraction, Response, StdError,
    StdResult, Uint128,
};
use osmosis_std::{
    cosmwasm_to_proto_coins,
    types::osmosis::{
        gamm::v1beta1::{GammQuerier, Pool},
        poolmanager::v1beta1::PoolmanagerQuerier,
    },
};
use valence_osmosis_utils::utils::{
    get_provide_liquidity_msg, get_provide_ss_liquidity_msg, query_pool_asset_balance,
};
use valence_service_utils::{error::ServiceError, execute_on_behalf_of};

use crate::{msg::LiquidityProviderConfig, valence_service_integration::Config};

pub fn provide_single_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
    asset: String,
    limit: Uint128,
) -> Result<Response, ServiceError> {
    // first we assert the input account balance
    let input_acc_asset_bal = query_pool_asset_balance(&deps, cfg.input_addr.as_str(), &asset)?;

    let pm_querier = PoolmanagerQuerier::new(&deps.querier);
    let pool = pm_querier.get_pool_response(&cfg.lp_config)?;

    // if the input balance is greater than the limit, we provision the limit amount.
    // otherwise we provision the full input balance.
    let provision_amount = if input_acc_asset_bal.amount > limit {
        limit
    } else {
        input_acc_asset_bal.amount
    };

    let share_out_amt = calculate_share_out_amt_swap(
        &deps,
        cfg.lp_config.pool_id,
        coins(provision_amount.u128(), asset.to_string()),
    )?;

    let liquidity_provision_msg = get_provide_ss_liquidity_msg(
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_id,
        coin(provision_amount.u128(), asset),
        share_out_amt.to_string(),
    )?;

    let transfer_lp_tokens_msg = BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: coins(
            share_out_amt.u128(),
            pool.total_shares
                .ok_or_else(|| StdError::generic_err("failed to get shares"))?
                .denom,
        ),
    };

    let delegated_input_acc_msgs = execute_on_behalf_of(
        vec![liquidity_provision_msg, transfer_lp_tokens_msg.into()],
        &cfg.input_addr.clone(),
    )?;

    Ok(Response::default().add_message(delegated_input_acc_msgs))
}

pub fn provide_double_sided_liquidity(
    deps: DepsMut,
    cfg: Config,
) -> Result<Response, ServiceError> {
    // first we assert the input account balances
    let bal_asset_1 = deps
        .querier
        .query_balance(&cfg.input_addr, &cfg.lp_config.pool_asset_1)?;
    let bal_asset_2 = deps
        .querier
        .query_balance(&cfg.input_addr, &cfg.lp_config.pool_asset_2)?;

    let pm_querier = PoolmanagerQuerier::new(&deps.querier);

    let pool_ratio = pm_querier.query_spot_price(&cfg.lp_config)?;
    let pool = pm_querier.get_pool_response(&cfg.lp_config)?;

    let provision_coins = calculate_provision_amounts(bal_asset_1, bal_asset_2, pool_ratio)?;

    let share_out_amt =
        calculate_share_out_amt_no_swap(&deps, cfg.lp_config.pool_id, provision_coins.clone())?;

    let liquidity_provision_msg: CosmosMsg = get_provide_liquidity_msg(
        cfg.input_addr.as_str(),
        cfg.lp_config.pool_id,
        provision_coins,
        share_out_amt.to_string(),
    )?;

    let transfer_lp_tokens_msg = BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: coins(
            share_out_amt.u128(),
            pool.total_shares
                .ok_or_else(|| StdError::generic_err("failed to get shares"))?
                .denom,
        ),
    };

    let delegated_msgs = execute_on_behalf_of(
        vec![liquidity_provision_msg, transfer_lp_tokens_msg.into()],
        &cfg.input_addr.clone(),
    )?;

    Ok(Response::default().add_message(delegated_msgs))
}

pub trait ValenceLiquidPooler {
    fn query_spot_price(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Decimal>;
    fn get_pool_response(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Pool>;
}

impl ValenceLiquidPooler for PoolmanagerQuerier<'_, Empty> {
    fn query_spot_price(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Decimal> {
        let spot_price_response = self.spot_price(
            lp_config.pool_id,
            lp_config.pool_asset_1.to_string(),
            lp_config.pool_asset_2.to_string(),
        )?;

        let pool_ratio = Decimal::from_str(&spot_price_response.spot_price)?;

        Ok(pool_ratio)
    }

    fn get_pool_response(&self, lp_config: &LiquidityProviderConfig) -> StdResult<Pool> {
        let pool_response = self.pool(lp_config.pool_id)?;

        let pool: Pool = pool_response
            .pool
            .ok_or_else(|| StdError::generic_err("failed to get pool"))?
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode proto"))?;

        Ok(pool)
    }
}

pub fn calculate_share_out_amt_no_swap(
    deps: &DepsMut,
    pool_id: u64,
    coins_in: Vec<Coin>,
) -> StdResult<Uint128> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    let shares_out = gamm_querier
        .calc_join_pool_no_swap_shares(pool_id, cosmwasm_to_proto_coins(coins_in))?
        .shares_out;

    let shares_u128 = Uint128::from_str(&shares_out)?;

    Ok(shares_u128)
}

pub fn calculate_share_out_amt_swap(
    deps: &DepsMut,
    pool_id: u64,
    coin_in: Vec<Coin>,
) -> StdResult<Uint128> {
    let gamm_querier = GammQuerier::new(&deps.querier);
    let shares_out = gamm_querier
        .calc_join_pool_shares(pool_id, cosmwasm_to_proto_coins(coin_in))?
        .share_out_amount;

    let shares_u128 = Uint128::from_str(&shares_out)?;

    Ok(shares_u128)
}

pub fn calculate_provision_amounts(
    mut asset_1_bal: Coin,
    mut asset_2_bal: Coin,
    pool_ratio: Decimal,
) -> StdResult<Vec<Coin>> {
    // first we assume that we are going to provide all of asset_1 and up to all of asset_2
    // then we get the expected amount of asset_2 tokens to provide
    let expected_asset_2_provision_amt = asset_1_bal
        .amount
        .checked_multiply_ratio(pool_ratio.numerator(), pool_ratio.denominator())
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    // then we check if the expected amount of asset_2 tokens is greater than the available balance
    if expected_asset_2_provision_amt > asset_2_bal.amount {
        // if it is, we calculate the amount of asset_1 tokens to provide
        asset_1_bal.amount = asset_2_bal
            .amount
            .checked_multiply_ratio(pool_ratio.denominator(), pool_ratio.numerator())
            .map_err(|e| StdError::generic_err(e.to_string()))?;
    } else {
        // if it is not, we provide all of asset_1 and the expected amount of asset_2
        asset_2_bal.amount = expected_asset_2_provision_amt;
    }

    Ok(vec![asset_1_bal, asset_2_bal])
}
