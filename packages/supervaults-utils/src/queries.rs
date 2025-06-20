use cosmwasm_std::{Addr, Deps, Uint128};
use neutron_std::types::neutron::util::precdec::PrecDec;
use valence_library_utils::error::LibraryError;

pub fn query_vault_price(deps: Deps, vault_addr: String) -> Result<PrecDec, LibraryError> {
    let price_response: mmvault::msg::CombinedPriceResponse = deps
        .querier
        .query_wasm_smart(vault_addr, &mmvault::msg::QueryMsg::GetPrices {})?;

    Ok(price_response.price_0_to_1)
}

pub fn query_simulate_provide_liquidity(
    deps: Deps,
    vault_addr: String,
    sender: Addr,
    amount_0: Uint128,
    amount_1: Uint128,
) -> Result<Uint128, LibraryError> {
    let simulate_response: Uint128 = deps.querier.query_wasm_smart(
        vault_addr,
        &mmvault::msg::QueryMsg::SimulateProvideLiquidity {
            amount_0,
            amount_1,
            sender,
        },
    )?;

    Ok(simulate_response)
}

pub fn query_simulate_withdraw_liquidity(
    deps: Deps,
    vault_addr: String,
    amount: Uint128,
) -> Result<(Uint128, Uint128), LibraryError> {
    let simulate_response: (Uint128, Uint128) = deps.querier.query_wasm_smart(
        vault_addr,
        &mmvault::msg::QueryMsg::SimulateWithdrawLiquidity { amount },
    )?;

    Ok(simulate_response)
}
