use astroport::asset::{Asset, PairInfo};
use cosmwasm_std::{to_json_binary, Coin, CosmosMsg, DepsMut};
use valence_service_utils::error::ServiceError;

use crate::msg::Config;

pub fn query_liquidity_token(deps: &DepsMut, cfg: &Config) -> Result<String, ServiceError> {
    let pair_info: PairInfo = deps
        .querier
        .query_wasm_smart(cfg.pool_addr.clone(), &astroport::pair::QueryMsg::Pair {})?;

    Ok(pair_info.liquidity_token)
}

pub fn create_withdraw_liquidity_msgs(
    deps: &DepsMut,
    cfg: &Config,
) -> Result<Vec<CosmosMsg>, ServiceError> {
    // Get the token factory token that represents the liquidity token
    let token = query_liquidity_token(deps, cfg)?;

    // Query the balance of the account that is going to withdraw
    let balance = deps.querier.query_balance(&cfg.input_addr, &token)?;
    if balance.amount.is_zero() {
        return Err(ServiceError::ExecutionError(
            "Nothing to withdraw".to_string(),
        ));
    }

    // Calculate how much we are going to get when we withdraw
    let withdrawn_assets: Vec<Asset> = deps.querier.query_wasm_smart(
        cfg.pool_addr.clone(),
        &astroport::pair::QueryMsg::Share {
            amount: balance.amount,
        },
    )?;

    // Create the withdraw and send messages
    let withdraw_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
        contract_addr: cfg.pool_addr.to_string(),
        msg: to_json_binary(&astroport::pair::ExecuteMsg::WithdrawLiquidity {
            assets: vec![],
            min_assets_to_receive: Some(withdrawn_assets.clone()),
        })?,
        funds: vec![balance],
    });

    // Send the withdrawn assets to the output account
    let withdrawn_coins = withdrawn_assets
        .into_iter()
        .map(|asset| asset.as_coin())
        .collect::<Result<Vec<Coin>, _>>()?;

    let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: withdrawn_coins,
    });

    Ok(vec![withdraw_msg, send_msg])
}
