use std::error::Error;

use valence_astroport_lper::msg::LiquidityProviderConfig;

use valence_authorization_utils::builders::AtomicFunctionBuilder;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
};
use valence_library_utils::liquidity_utils::AssetData;
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
