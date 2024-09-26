use astroport_cw20_lp_token::asset::{Asset, PairInfo};
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, DepsMut};
use cw20::{BalanceResponse, Cw20ExecuteMsg};

use crate::{error::ServiceError, msg::Config};

pub fn query_liquidity_token(deps: &DepsMut, cfg: &Config) -> Result<Addr, ServiceError> {
    let pair_info: PairInfo = deps.querier.query_wasm_smart(
        cfg.pool_addr.clone(),
        &astroport_cw20_lp_token::pair::QueryMsg::Pair {},
    )?;

    Ok(pair_info.liquidity_token)
}

pub fn create_withdraw_liquidity_msgs(
    deps: &DepsMut,
    cfg: &Config,
) -> Result<Vec<CosmosMsg>, ServiceError> {
    // Get the token factory token that represents the liquidity token
    let token_addr = query_liquidity_token(deps, cfg)?;

    // Query the balance of the account that is going to withdraw
    let balance_response: BalanceResponse = deps.querier.query_wasm_smart(
        token_addr.clone(),
        &cw20::Cw20QueryMsg::Balance {
            address: cfg.input_addr.to_string(),
        },
    )?;
    if balance_response.balance.is_zero() {
        return Err(ServiceError::ExecutionError(
            "Nothing to withdraw".to_string(),
        ));
    }

    // Calculate how much we are going to get when we withdraw
    let withdrawn_assets: Vec<Asset> = deps.querier.query_wasm_smart(
        cfg.pool_addr.clone(),
        &astroport_cw20_lp_token::pair::QueryMsg::Share {
            amount: balance_response.balance,
        },
    )?;

    // Create the withdraw and send messages
    let withdraw_msg = CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
        contract_addr: token_addr.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Send {
            contract: cfg.pool_addr.to_string(),
            amount: balance_response.balance,
            msg: to_json_binary(
                &astroport_cw20_lp_token::pair::Cw20HookMsg::WithdrawLiquidity { assets: vec![] },
            )?,
        })?,
        funds: vec![],
    });

    // Send the withdrawn assets to the output account
    let withdrawn_coins = withdrawn_assets
        .into_iter()
        .map(|asset| asset.to_coin())
        .collect::<Result<Vec<Coin>, _>>()?;

    let send_msg = CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
        to_address: cfg.output_addr.to_string(),
        amount: withdrawn_coins,
    });

    Ok(vec![withdraw_msg, send_msg])
}
