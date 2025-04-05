use std::collections::BTreeMap;
use std::error::Error;
use std::time::Duration;

use cosmwasm_std::{Binary, Uint128};
use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::utils::test_context::TestContext;
use localic_utils::{
    DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_DENOM, NEUTRON_CHAIN_NAME,
};
use log::info;
use valence_astroport_lper::msg::LiquidityProviderConfig;

use valence_e2e::utils::base_account::{approve_library, create_base_accounts};
use valence_e2e::utils::hyperlane::HyperlaneContracts;
use valence_e2e::utils::manager::{
    ASTROPORT_LPER_NAME, ASTROPORT_WITHDRAWER_NAME, BASE_ACCOUNT_NAME, ICA_CCTP_TRANSFER_NAME,
    ICA_IBC_TRANSFER_NAME, NEUTRON_IBC_TRANSFER_NAME,
};
use valence_e2e::utils::{NOBLE_CHAIN_NAME, UUSDC_DENOM};
use valence_generic_ibc_transfer_library::msg::IbcTransferAmount;
use valence_ica_ibc_transfer::msg::RemoteChainInfo;
use valence_library_utils::liquidity_utils::AssetData;
use valence_library_utils::LibraryAccountType;

use crate::neutron::ica::{instantiate_interchain_account_contract, register_interchain_account};
use crate::ASTROPORT_CONCENTRATED_PAIR_TYPE;

#[derive(Debug, Clone)]
pub struct ValenceInterchainAccount {
    pub library_account: LibraryAccountType,
    pub remote_addr: String,
}

pub struct NeutronProgramAccounts {
    pub deposit_account: LibraryAccountType,
    pub position_account: LibraryAccountType,
    pub withdraw_account: LibraryAccountType,
    pub noble_inbound_ica: ValenceInterchainAccount,
    pub noble_outbound_ica: ValenceInterchainAccount,
}

pub struct NeutronProgramLibraries {
    pub astroport_lper: String,
    pub astroport_lwer: String,
    pub noble_inbound_transfer: String,
    pub noble_cctp_transfer: String,
    pub neutron_ibc_transfer: String,
}

#[allow(unused)]
pub struct ProgramHyperlaneContracts {
    pub neutron_hyperlane_contracts: HyperlaneContracts,
    pub eth_hyperlane_contracts: HyperlaneContracts,
}

pub fn setup_neutron_accounts(
    test_ctx: &mut TestContext,
) -> Result<NeutronProgramAccounts, Box<dyn Error>> {
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
        3,
        None,
    );

    let deposit_account_addr = neutron_base_accounts[0].to_string();
    let position_account_addr = neutron_base_accounts[1].to_string();
    let withdraw_account_addr = neutron_base_accounts[2].to_string();

    let deposit_account = LibraryAccountType::Addr(deposit_account_addr.to_string());
    let position_account = LibraryAccountType::Addr(position_account_addr.to_string());
    let withdraw_account = LibraryAccountType::Addr(withdraw_account_addr.to_string());

    let noble_inbound_interchain_account_addr = instantiate_interchain_account_contract(test_ctx)?;

    let inbound_noble_ica_addr =
        register_interchain_account(test_ctx, &noble_inbound_interchain_account_addr)?;

    let noble_inbound_ica = ValenceInterchainAccount {
        library_account: LibraryAccountType::Addr(noble_inbound_interchain_account_addr),
        remote_addr: inbound_noble_ica_addr,
    };

    let noble_outbound_interchain_account_addr = instantiate_interchain_account_contract(test_ctx)?;

    let outbound_noble_ica_addr =
        register_interchain_account(test_ctx, &noble_outbound_interchain_account_addr)?;

    let noble_outbound_ica = ValenceInterchainAccount {
        library_account: LibraryAccountType::Addr(noble_outbound_interchain_account_addr),
        remote_addr: outbound_noble_ica_addr,
    };

    let neutron_accounts = NeutronProgramAccounts {
        // base accounts
        deposit_account,
        position_account,
        withdraw_account,
        // valence-icas
        noble_inbound_ica,
        noble_outbound_ica,
    };

    Ok(neutron_accounts)
}

pub fn setup_neutron_libraries(
    test_ctx: &mut TestContext,
    neutron_program_accounts: &NeutronProgramAccounts,
    pool: &str,
    processor: &str,
    amount: u128,
    usdc_on_neutron: &str,
    eth_withdraw_acc: String,
) -> Result<NeutronProgramLibraries, Box<dyn Error>> {
    let astro_cl_pool_asset_data = AssetData {
        asset1: NEUTRON_CHAIN_DENOM.to_string(),
        asset2: usdc_on_neutron.to_string(),
    };

    // library to enter into the position from the deposit account
    // and route the issued shares into the into the position account
    let astro_lper_lib = setup_astroport_lper_lib(
        test_ctx,
        neutron_program_accounts.deposit_account.clone(),
        neutron_program_accounts.position_account.clone(),
        astro_cl_pool_asset_data.clone(),
        pool.to_string(),
        processor.to_string(),
    )?;

    // library to withdraw the position held by the position account
    // and route the underlying funds into the withdraw account
    let astro_lwer_lib = setup_astroport_lwer_lib(
        test_ctx,
        neutron_program_accounts.position_account.clone(),
        neutron_program_accounts.withdraw_account.clone(),
        astro_cl_pool_asset_data.clone(),
        pool.to_string(),
        processor.to_string(),
    )?;

    // library to move USDC from a program-owned ICA on noble
    // into the deposit account on neutron
    let ica_ibc_transfer_lib = setup_ica_ibc_transfer_lib(
        test_ctx,
        &neutron_program_accounts
            .noble_inbound_ica
            .library_account
            .to_string()?,
        &neutron_program_accounts.deposit_account.to_string()?,
        amount,
    )?;

    // library to move USDC from a program-owned ICA on noble
    // into the withdraw account on ethereum
    let cctp_forwarder_lib_addr = setup_cctp_forwarder_lib(
        test_ctx,
        neutron_program_accounts
            .noble_outbound_ica
            .library_account
            .clone(),
        eth_withdraw_acc,
        processor.to_string(),
        amount,
    )?;

    // library to move USDC from the withdraw account on neutron
    // into a program-owned ICA on noble
    let neutron_ibc_transfer_lib = setup_neutron_ibc_transfer_lib(
        test_ctx,
        neutron_program_accounts.withdraw_account.clone(),
        valence_library_utils::LibraryAccountType::Addr(
            neutron_program_accounts
                .noble_outbound_ica
                .remote_addr
                .to_string(),
        ),
        usdc_on_neutron,
    )?;

    info!("approving strategist on withdraw account...");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &neutron_program_accounts
            .withdraw_account
            .to_string()
            .unwrap(),
        NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        None,
    );

    let libraries = NeutronProgramLibraries {
        astroport_lper: astro_lper_lib,
        astroport_lwer: astro_lwer_lib,
        noble_inbound_transfer: ica_ibc_transfer_lib,
        noble_cctp_transfer: cctp_forwarder_lib_addr,
        neutron_ibc_transfer: neutron_ibc_transfer_lib,
    };

    Ok(libraries)
}

pub fn setup_astroport_lper_lib(
    test_ctx: &mut TestContext,
    input_account: LibraryAccountType,
    output_account: LibraryAccountType,
    asset_data: AssetData,
    pool_addr: String,
    _processor: String,
) -> Result<String, Box<dyn Error>> {
    let lper_code_id = test_ctx
        .get_contract()
        .contract(ASTROPORT_LPER_NAME)
        .get_cw()
        .code_id
        .unwrap();

    let astro_cl_pair_type = valence_astroport_utils::astroport_native_lp_token::PairType::Custom(
        ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string(),
    );

    let astro_lp_config = LiquidityProviderConfig {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(astro_cl_pair_type.clone()),
        asset_data,
        max_spread: None,
    };

    let astro_lper_library_cfg = valence_astroport_lper::msg::LibraryConfig {
        input_addr: input_account.clone(),
        output_addr: output_account.clone(),
        lp_config: astro_lp_config,
        pool_addr,
    };

    let astroport_lper_instantiate_msg =
        valence_library_utils::msg::InstantiateMsg::<valence_astroport_lper::msg::LibraryConfig> {
            owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
            processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
            config: astro_lper_library_cfg,
        };

    let astro_lper_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        lper_code_id,
        &serde_json::to_string(&astroport_lper_instantiate_msg)?,
        "astro_lper",
        None,
        "",
    )?;
    info!("astro lper lib: {}", astro_lper_lib.address);

    info!("approving astro lper library on deposit account...");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account.to_string()?,
        astro_lper_lib.address.to_string(),
        None,
    );

    Ok(astro_lper_lib.address)
}

pub fn setup_astroport_lwer_lib(
    test_ctx: &mut TestContext,
    input_account: LibraryAccountType,
    output_account: LibraryAccountType,
    asset_data: AssetData,
    pool_addr: String,
    _processor: String,
) -> Result<String, Box<dyn Error>> {
    let lwer_code_id = test_ctx
        .get_contract()
        .contract(ASTROPORT_WITHDRAWER_NAME)
        .get_cw()
        .code_id
        .unwrap();

    let astro_cl_pair_type = valence_astroport_utils::astroport_native_lp_token::PairType::Custom(
        ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string(),
    );

    let astro_lw_config = valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(astro_cl_pair_type),
        asset_data,
    };
    let astro_lwer_library_cfg = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: input_account.clone(),
        output_addr: output_account.clone(),
        withdrawer_config: astro_lw_config,
        pool_addr: pool_addr.to_string(),
    };
    let astroport_lwer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_astroport_withdrawer::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: astro_lwer_library_cfg,
    };

    let astro_lwer_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        lwer_code_id,
        &serde_json::to_string(&astroport_lwer_instantiate_msg)?,
        "astro_lwer",
        None,
        "",
    )?;
    info!("astro lwer lib: {}", astro_lwer_lib.address);

    info!("approving astro lwer library on position account...");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account.to_string()?,
        astro_lwer_lib.address.to_string(),
        None,
    );

    Ok(astro_lwer_lib.address)
}

pub fn setup_cctp_forwarder_lib(
    test_ctx: &mut TestContext,
    input_account: LibraryAccountType,
    mut output_addr: String,
    _processor: String,
    amount: u128,
) -> Result<String, Box<dyn Error>> {
    let ica_cctp_transfer_code_id = test_ctx
        .get_contract()
        .contract(ICA_CCTP_TRANSFER_NAME)
        .get_cw()
        .code_id
        .unwrap();

    let trimmed_addr = output_addr.split_off(2);
    let mut mint_recipient = vec![0u8; 32];

    let addr_bytes = hex::decode(trimmed_addr).unwrap();
    mint_recipient[(32 - addr_bytes.len())..].copy_from_slice(&addr_bytes);

    let cctp_transfer_config = valence_ica_cctp_transfer::msg::LibraryConfig {
        input_addr: input_account.clone(),
        amount: (amount / 2).into(),
        denom: UUSDC_DENOM.to_string(),
        destination_domain_id: 0,
        mint_recipient: Binary::from(mint_recipient),
    };

    let ica_cctp_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_ica_cctp_transfer::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: cctp_transfer_config,
    };

    let cctp_transfer_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        ica_cctp_transfer_code_id,
        &serde_json::to_string(&ica_cctp_transfer_instantiate_msg)?,
        "cctp_transfer",
        None,
        "",
    )?;
    info!("cctp transfer lib: {}", cctp_transfer_lib.address);

    info!("approving cctp transfer library on account...");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account.to_string()?,
        cctp_transfer_lib.address.to_string(),
        None,
    );

    Ok(cctp_transfer_lib.address)
}

pub fn setup_ica_ibc_transfer_lib(
    test_ctx: &mut TestContext,
    interchain_account_addr: &str,
    neutron_deposit_acc: &str,
    amount_to_transfer: u128,
) -> Result<String, Box<dyn Error>> {
    let ica_ibc_transfer_lib_code = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get(ICA_IBC_TRANSFER_NAME)
        .unwrap();

    info!("ica ibc transfer lib code: {ica_ibc_transfer_lib_code}");

    info!("Instantiating the ICA IBC transfer contract...");
    let ica_ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_ica_ibc_transfer::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: valence_ica_ibc_transfer::msg::LibraryConfig {
            input_addr: LibraryAccountType::Addr(interchain_account_addr.to_string()),
            amount: Uint128::new(amount_to_transfer),
            denom: UUSDC_DENOM.to_string(),
            receiver: neutron_deposit_acc.to_string(),
            memo: "".to_string(),
            remote_chain_info: RemoteChainInfo {
                channel_id: test_ctx
                    .get_transfer_channels()
                    .src(NOBLE_CHAIN_NAME)
                    .dest(NEUTRON_CHAIN_NAME)
                    .get(),
                ibc_transfer_timeout: None,
            },
            denom_to_pfm_map: BTreeMap::default(),
        },
    };

    let ica_ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        ica_ibc_transfer_lib_code,
        &serde_json::to_string(&ica_ibc_transfer_instantiate_msg)?,
        "valence_ica_ibc_transfer",
        None,
        "",
    )?;
    info!(
        "ICA IBC transfer contract instantiated. Address: {}",
        ica_ibc_transfer.address
    );

    info!("Approving the ICA IBC transfer library...");
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        interchain_account_addr,
        ica_ibc_transfer.address.to_string(),
        None,
    );

    std::thread::sleep(Duration::from_secs(2));

    Ok(ica_ibc_transfer.address)
}

pub fn setup_neutron_ibc_transfer_lib(
    test_ctx: &mut TestContext,
    input_account: LibraryAccountType,
    output_addr: LibraryAccountType,
    denom: &str,
) -> Result<String, Box<dyn Error>> {
    let neutron_ibc_transfer_code_id = *test_ctx
        .get_chain(NEUTRON_CHAIN_NAME)
        .contract_codes
        .get(NEUTRON_IBC_TRANSFER_NAME)
        .unwrap();

    let neutron_ibc_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_neutron_ibc_transfer_library::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        config: valence_neutron_ibc_transfer_library::msg::LibraryConfig {
            input_addr: input_account.clone(),
            amount: IbcTransferAmount::FullAmount,
            denom: valence_library_utils::denoms::UncheckedDenom::Native(denom.to_string()),
            remote_chain_info: valence_generic_ibc_transfer_library::msg::RemoteChainInfo {
                channel_id: test_ctx
                    .get_transfer_channels()
                    .src(NEUTRON_CHAIN_NAME)
                    .dest(NOBLE_CHAIN_NAME)
                    .get(),
                ibc_transfer_timeout: None,
            },
            output_addr: output_addr.clone(),
            memo: "-".to_string(),
            denom_to_pfm_map: BTreeMap::default(),
        },
    };

    info!(
        "Neutron IBC Transfer instantiate message: {:?}",
        neutron_ibc_transfer_instantiate_msg
    );

    let ibc_transfer = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        neutron_ibc_transfer_code_id,
        &serde_json::to_string(&neutron_ibc_transfer_instantiate_msg).unwrap(),
        "neutron_ibc_transfer",
        None,
        "",
    )
    .unwrap();

    info!(
        "Neutron IBC Transfer library: {}",
        ibc_transfer.address.clone()
    );

    // Approve the library for the base account
    approve_library(
        test_ctx,
        NEUTRON_CHAIN_NAME,
        DEFAULT_KEY,
        &input_account.to_string()?,
        ibc_transfer.address.clone(),
        None,
    );

    Ok(ibc_transfer.address)
}
