use std::error::Error;

////////////////////////////////////////////
// DECLARE TEST ENVIRONMENT CONFIGURATION //
////////////////////////////////////////////

// import e2e test utilities

use localic_utils::OSMOSIS_CHAIN_NAME;
use log::info;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
};
use valence_library_utils::liquidity_utils::AssetData;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

pub fn my_osmosis_gamm_program(
    osmo_domain: valence_program_manager::domain::Domain,
    owner: String,
    pool_id: u64,
    denom_1: &str,
    denom_2: &str,
) -> Result<ProgramConfig, Box<dyn Error>> {
    let mut builder = ProgramConfigBuilder::new(owner);

    let gamm_input_acc_info = AccountInfo::new(
        "gamm_input".to_string(),
        &osmo_domain,
        AccountType::default(),
    );
    let gamm_output_acc_info = AccountInfo::new(
        "gamm_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );
    let final_output_acc_info = AccountInfo::new(
        "final_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    let gamm_input_acc = builder.add_account(gamm_input_acc_info);
    let gamm_output_acc = builder.add_account(gamm_output_acc_info);
    let final_output_acc = builder.add_account(final_output_acc_info);

    info!("gamm input acc: {:?}", gamm_input_acc);
    info!("gamm output acc: {:?}", gamm_output_acc);
    info!("final output acc: {:?}", final_output_acc);

    let gamm_lper_config = valence_osmosis_gamm_lper::msg::LibraryConfig {
        input_addr: gamm_input_acc.clone(),
        output_addr: gamm_output_acc.clone(),
        lp_config: valence_osmosis_gamm_lper::msg::LiquidityProviderConfig {
            pool_id,
            asset_data: AssetData {
                asset1: denom_1.to_string(),
                asset2: denom_2.to_string(),
            },
        },
    };

    let gamm_lwer_config = valence_osmosis_gamm_withdrawer::msg::LibraryConfig {
        input_addr: gamm_output_acc.clone(),
        output_addr: final_output_acc.clone(),
        lw_config: valence_osmosis_gamm_withdrawer::msg::LiquidityWithdrawerConfig {
            pool_id,
            asset_data: AssetData {
                asset1: denom_1.to_string(),
                asset2: denom_2.to_string(),
            },
        },
    };

    let gamm_lper_library_info = LibraryInfo::new(
        "test_gamm_lp".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisGammLper(gamm_lper_config),
    );
    let gamm_lper_library = builder.add_library(gamm_lper_library_info);

    let gamm_lwer_library_info = LibraryInfo::new(
        "test_gamm_lw".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisGammWithdrawer(gamm_lwer_config),
    );
    let gamm_lwer_library = builder.add_library(gamm_lwer_library_info);

    // establish the input_acc -> lper_lib -> output_acc link
    builder.add_link(
        &gamm_lper_library,
        vec![&gamm_input_acc],
        vec![&gamm_output_acc],
    );
    // establish the output_acc -> lwer_lib -> final_output_acc link
    builder.add_link(
        &gamm_lwer_library,
        vec![&gamm_output_acc],
        vec![&final_output_acc],
    );

    let gamm_lper_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(gamm_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let gamm_lwer_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(gamm_lwer_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    let gamm_lper_atomic_subroutine = AtomicSubroutineBuilder::new()
        .with_function(gamm_lper_function)
        .build();

    let gamm_lper_authorization = AuthorizationBuilder::new()
        .with_label("provide_liquidity")
        .with_subroutine(gamm_lper_atomic_subroutine)
        .build();

    let gamm_lwer_atomic_subroutine = AtomicSubroutineBuilder::new()
        .with_function(gamm_lwer_function)
        .build();

    let gamm_lwer_authorization = AuthorizationBuilder::new()
        .with_label("withdraw_liquidity")
        .with_subroutine(gamm_lwer_atomic_subroutine)
        .build();

    builder.add_authorization(gamm_lper_authorization);
    builder.add_authorization(gamm_lwer_authorization);

    Ok(builder.build())
}
