use cosmwasm_std::{Coin, Deps, StdResult};

use crate::{ATOM_DENOM, NEWT_DENOM, NTRN_DENOM, USDC_DENOM};

pub fn get_balances(deps: Deps, addr: String) -> StdResult<Vec<Coin>> {
    let mut balances = Vec::with_capacity(4);

    let ntrn_balance = deps
        .querier
        .query_balance(addr.clone(), NTRN_DENOM.to_string())?;
    let newt_balance = deps
        .querier
        .query_balance(addr.clone(), NEWT_DENOM.to_string())?;
    let usdc_balance = deps
        .querier
        .query_balance(addr.clone(), USDC_DENOM.to_string())?;
    let atom_balance = deps
        .querier
        .query_balance(addr.clone(), ATOM_DENOM.to_string())?;

    if !ntrn_balance.amount.is_zero() {
        balances.push(ntrn_balance);
    }
    if !newt_balance.amount.is_zero() {
        balances.push(newt_balance);
    }
    if !usdc_balance.amount.is_zero() {
        balances.push(usdc_balance);
    }
    if !atom_balance.amount.is_zero() {
        balances.push(atom_balance);
    }

    Ok(balances)
}
