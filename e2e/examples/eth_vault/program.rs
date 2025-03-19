use std::error::Error;

use cosmwasm_std::Binary;
use localic_std::modules::cosmwasm::contract_instantiate;
use localic_utils::utils::test_context::TestContext;
use localic_utils::{DEFAULT_KEY, NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME};
use log::info;
use valence_astroport_lper::msg::LiquidityProviderConfig;

use valence_authorization_utils::builders::AtomicFunctionBuilder;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
};
use valence_e2e::utils::base_account::{approve_library, create_base_accounts};
use valence_e2e::utils::UUSDC_DENOM;
use valence_library_utils::liquidity_utils::AssetData;
use valence_library_utils::LibraryAccountType;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::LibraryInfo,
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

use crate::{
    ASTROPORT_CONCENTRATED_PAIR_TYPE, PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL,
    WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL,
};

pub fn my_evm_vault_program(
    ntrn_domain: valence_program_manager::domain::Domain,
    asset_1: &str,
    asset_2: &str,
    pool_addr: &str,
    owner: &str,
) -> Result<ProgramConfig, Box<dyn Error>> {
    let mut builder = ProgramConfigBuilder::new("vault test", owner);

    let deposit_account_info =
        AccountInfo::new("deposit".to_string(), &ntrn_domain, AccountType::default());

    let position_account_info =
        AccountInfo::new("position".to_string(), &ntrn_domain, AccountType::default());

    let withdraw_account_info =
        AccountInfo::new("withdraw".to_string(), &ntrn_domain, AccountType::default());

    let deposit_acc = builder.add_account(deposit_account_info);
    let position_acc = builder.add_account(position_account_info);
    let withdraw_acc = builder.add_account(withdraw_account_info);

    let astro_cl_pair_type = valence_astroport_utils::astroport_native_lp_token::PairType::Custom(
        ASTROPORT_CONCENTRATED_PAIR_TYPE.to_string(),
    );

    let astro_cl_pool_asset_data = AssetData {
        asset1: asset_1.to_string(),
        asset2: asset_2.to_string(),
    };

    let astro_lp_config = LiquidityProviderConfig {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(astro_cl_pair_type.clone()),
        asset_data: astro_cl_pool_asset_data.clone(),
        max_spread: None,
    };

    let astro_lw_config = valence_astroport_withdrawer::msg::LiquidityWithdrawerConfig {
        pool_type: valence_astroport_utils::PoolType::NativeLpToken(astro_cl_pair_type),
        asset_data: astro_cl_pool_asset_data.clone(),
    };

    let astro_lper_library_cfg = valence_astroport_lper::msg::LibraryConfig {
        input_addr: deposit_acc.clone(),
        output_addr: position_acc.clone(),
        lp_config: astro_lp_config,
        pool_addr: pool_addr.to_string(),
    };
    let astro_lwer_library_cfg = valence_astroport_withdrawer::msg::LibraryConfig {
        input_addr: position_acc.clone(),
        output_addr: withdraw_acc.clone(),
        withdrawer_config: astro_lw_config,
        pool_addr: pool_addr.to_string(),
    };

    let astro_lper_library = builder.add_library(LibraryInfo::new(
        "astro_lp".to_string(),
        &ntrn_domain,
        valence_program_manager::library::LibraryConfig::ValenceAstroportLper(
            astro_lper_library_cfg,
        ),
    ));

    let astro_lwer_library = builder.add_library(LibraryInfo::new(
        "astro_lw".to_string(),
        &ntrn_domain,
        valence_program_manager::library::LibraryConfig::ValenceAstroportWithdrawer(
            astro_lwer_library_cfg,
        ),
    ));

    // establish the deposit_acc -> lper_lib -> position_acc link
    builder.add_link(&astro_lper_library, vec![&deposit_acc], vec![&position_acc]);
    // establish the position_acc -> lwer_lib -> withdraw_acc link
    builder.add_link(
        &astro_lwer_library,
        vec![&position_acc],
        vec![&withdraw_acc],
    );

    let astro_lper_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::Main)
        .with_contract_address(astro_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let astro_lwer_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::Main)
        .with_contract_address(astro_lwer_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let astro_lper_subroutine = AtomicSubroutineBuilder::new()
        .with_function(astro_lper_function)
        .build();

    let astro_lwer_subroutine = AtomicSubroutineBuilder::new()
        .with_function(astro_lwer_function)
        .build();

    let astro_lper_authorization = AuthorizationBuilder::new()
        .with_label(PROVIDE_LIQUIDITY_AUTHORIZATIONS_LABEL)
        .with_subroutine(astro_lper_subroutine)
        .build();
    let astro_lwer_authorization = AuthorizationBuilder::new()
        .with_label(WITHDRAW_LIQUIDITY_AUTHORIZATIONS_LABEL)
        .with_subroutine(astro_lwer_subroutine)
        .build();

    builder.add_authorization(astro_lper_authorization);
    builder.add_authorization(astro_lwer_authorization);

    let program_config = builder.build();

    Ok(program_config)
}

pub fn setup_neutron_accounts(
    test_ctx: &mut TestContext,
    base_account_code_id: u64,
) -> Result<(LibraryAccountType, LibraryAccountType, LibraryAccountType), Box<dyn Error>> {
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

    Ok((deposit_account, position_account, withdraw_account))
}

pub fn setup_astroport_lper_lib(
    test_ctx: &mut TestContext,
    input_account: LibraryAccountType,
    output_account: LibraryAccountType,
    asset_data: AssetData,
    pool_addr: String,
    processor: String,
    lper_code_id: u64,
) -> Result<String, Box<dyn Error>> {
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
            processor,
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
    processor: String,
    lwer_code_id: u64,
) -> Result<String, Box<dyn Error>> {
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
        processor: processor.to_string(),
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
    output_addr: String,
    processor: String,
    cctp_transfer_code_id: u64,
    amount: u128,
) -> Result<String, Box<dyn Error>> {
    let cctp_transfer_config = valence_ica_cctp_transfer::msg::LibraryConfig {
        input_addr: input_account.clone(),
        amount: amount.into(),
        denom: UUSDC_DENOM.to_string(),
        destination_domain_id: 0,
        mint_recipient: Binary::from(&[0x01; 32]), // TODO
    };

    let ica_cctp_transfer_instantiate_msg = valence_library_utils::msg::InstantiateMsg::<
        valence_ica_cctp_transfer::msg::LibraryConfig,
    > {
        owner: NEUTRON_CHAIN_ADMIN_ADDR.to_string(),
        processor: processor.to_string(),
        config: cctp_transfer_config,
    };

    let cctp_transfer_lib = contract_instantiate(
        test_ctx
            .get_request_builder()
            .get_request_builder(NEUTRON_CHAIN_NAME),
        DEFAULT_KEY,
        cctp_transfer_code_id,
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
