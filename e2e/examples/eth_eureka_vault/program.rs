use std::{error::Error, path::Path};

use localic_utils::{
    utils::test_context::TestContext, DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM,
    NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_e2e::utils::{
    astroport::{setup_astroport_lper_lib, setup_astroport_lwer_lib},
    base_account::{approve_library, create_base_accounts},
    manager::{
        ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME, BASE_ACCOUNT_NAME, FORWARDER_NAME,
        NEUTRON_IBC_TRANSFER_NAME,
    },
    vault::{setup_liquidation_fwd_lib, setup_neutron_ibc_transfer_lib},
    LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
};
use valence_library_utils::liquidity_utils::AssetData;

use crate::{strategist::strategy_config, VAULT_NEUTRON_CACHE_PATH};

pub fn upload_neutron_contracts(test_ctx: &mut TestContext) -> Result<(), Box<dyn Error>> {
    // copy over relevant contracts from artifacts/ to local path
    let local_contracts_path = Path::new(VAULT_NEUTRON_CACHE_PATH);
    if !local_contracts_path.exists() {
        std::fs::create_dir(local_contracts_path)?;
    }

    for contract in [
        ASTROPORT_LPER_NAME,
        ASTROPORT_WITHDRAWER_NAME,
        NEUTRON_IBC_TRANSFER_NAME,
        FORWARDER_NAME,
        BASE_ACCOUNT_NAME,
    ] {
        let contract_name = format!("{}.wasm", contract);
        let contract_path = Path::new(&contract_name);
        let src = Path::new("artifacts/").join(contract_path);
        let dest = local_contracts_path.join(contract_path);
        std::fs::copy(src, dest)?;
    }

    let mut uploader = test_ctx.build_tx_upload_contracts();
    uploader
        .with_chain_name(NEUTRON_CHAIN_NAME)
        .send_with_local_cache(
            "e2e/examples/eth_eureka_vault/neutron_contracts/",
            LOCAL_CODE_ID_CACHE_PATH_NEUTRON,
        )?;

    Ok(())
}

pub fn setup_neutron_accounts(
    test_ctx: &mut TestContext,
) -> Result<strategy_config::neutron::NeutronAccounts, Box<dyn Error>> {
    let base_account_code_id = test_ctx
        .get_contract()
        .contract(BASE_ACCOUNT_NAME)
        .get_cw()
        .code_id
        .unwrap();

    let neutron_base_accounts = create_base_accounts(
        test_ctx,
        DEFAULT_KEY,
        NEUTRON_CHAIN_NAME,
        base_account_code_id,
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        vec![],
        4,
        None,
    );

    let neutron_accounts = strategy_config::neutron::NeutronAccounts {
        deposit: neutron_base_accounts[0].to_string(),
        position: neutron_base_accounts[1].to_string(),
        withdraw: neutron_base_accounts[2].to_string(),
        liquidation: neutron_base_accounts[3].to_string(),
    };

    Ok(neutron_accounts)
}

#[allow(clippy::too_many_arguments)]
pub fn setup_neutron_libraries(
    test_ctx: &mut TestContext,
    neutron_program_accounts: &strategy_config::neutron::NeutronAccounts,
    pool: &str,
    authorizations: &str,
    processor: &str,
    wbtc_on_neutron: &str,
    eth_withdraw_acc: String,
    lp_token_denom: &str,
) -> Result<strategy_config::neutron::NeutronLibraries, Box<dyn Error>> {
    let astro_cl_pool_asset_data = AssetData {
        asset1: NEUTRON_CHAIN_DENOM.to_string(),
        asset2: wbtc_on_neutron.to_string(),
    };

    // library to enter into the position from the deposit account
    // and route the issued shares into the into the position account
    let astro_lper_lib = setup_astroport_lper_lib(
        test_ctx,
        neutron_program_accounts.deposit.to_string(),
        neutron_program_accounts.position.to_string(),
        astro_cl_pool_asset_data.clone(),
        pool.to_string(),
        processor.to_string(),
        authorizations.to_string(),
    )?;

    // library to forward the required amount of shares, from the position account
    // to the liquidation account, needed to fulfill the withdraw obligations
    let forwarder_lib = setup_liquidation_fwd_lib(
        test_ctx,
        neutron_program_accounts.position.to_string(),
        neutron_program_accounts.liquidation.to_string(),
        lp_token_denom,
    )?;

    // library to withdraw the position held by the position account
    // and route the underlying funds into the withdraw account
    let astro_lwer_lib = setup_astroport_lwer_lib(
        test_ctx,
        neutron_program_accounts.liquidation.to_string(),
        neutron_program_accounts.withdraw.to_string(),
        astro_cl_pool_asset_data.clone(),
        pool.to_string(),
        processor.to_string(),
    )?;

    info!("approving strategist on liquidation account...");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &neutron_program_accounts.liquidation.to_string(),
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        None,
    );

    // library to move USDC from the withdraw account on neutron
    // into a program-owned ICA on noble
    let neutron_ibc_transfer_lib = setup_neutron_ibc_transfer_lib(
        test_ctx,
        neutron_program_accounts.withdraw.to_string(),
        eth_withdraw_acc,
        wbtc_on_neutron,
        authorizations.to_string(),
        processor.to_string(),
    )?;

    info!("neutron ibc transfer library: {neutron_ibc_transfer_lib}");

    let libraries = strategy_config::neutron::NeutronLibraries {
        astroport_lper: astro_lper_lib,
        astroport_lwer: astro_lwer_lib,
        neutron_ibc_transfer: neutron_ibc_transfer_lib,
        liquidation_forwarder: forwarder_lib,
        authorizations: authorizations.to_string(),
        processor: processor.to_string(),
    };

    info!("neutron libraries: {:?}", libraries);

    Ok(libraries)
}
