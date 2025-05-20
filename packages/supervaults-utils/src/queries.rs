use cosmwasm_std::Deps;
use neutron_std::types::neutron::util::precdec::PrecDec;
use valence_library_utils::error::LibraryError;

pub fn query_vault_price(deps: Deps, vault_addr: String) -> Result<PrecDec, LibraryError> {
    let price_response: mmvault::msg::CombinedPriceResponse = deps
        .querier
        .query_wasm_smart(vault_addr, &mmvault::msg::QueryMsg::GetPrices {})?;

    Ok(price_response.price_0_to_1)
}
