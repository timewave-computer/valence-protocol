use cosmwasm_std::{ensure, Deps};
use valence_library_utils::{error::LibraryError, liquidity_utils::AssetData};

pub mod prec_dec_range;
pub mod queries;

pub fn ensure_correct_vault(
    deps: Deps,
    vault_addr: String,
    asset_data: &AssetData,
    lp_denom: &str,
) -> Result<(), LibraryError> {
    let vault_config: mmvault::state::Config = deps
        .querier
        .query_wasm_smart(vault_addr, &mmvault::msg::QueryMsg::GetConfig {})?;

    ensure!(
        asset_data.asset1 == vault_config.pair_data.token_0.denom
            && asset_data.asset2 == vault_config.pair_data.token_1.denom,
        LibraryError::ConfigurationError(
            "Pool type does not match the expected pair type".to_string(),
        )
    );

    ensure!(
        vault_config.lp_denom == lp_denom,
        LibraryError::ConfigurationError(format!(
            "Vault LP denom mismatch; expected: {lp_denom}, got {}",
            vault_config.lp_denom
        ))
    );

    Ok(())
}
