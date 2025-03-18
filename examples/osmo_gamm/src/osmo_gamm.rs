use localic_utils::OSMOSIS_CHAIN_NAME;
use std::error::Error;
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

/// Returns a `ProgramConfig` for a program that provides liquidity to a GAMM
/// liquidity pool on Osmosis. The position is held in a program controlled account until
/// it is withdrawn.
///
/// The returned `ProgramConfig` can be used to deploy and configure the program.
/// The accompanying osmo_gamm_test.rs demonstrates local deployment and testing of
/// this program.
///
/// # Arguments
/// * `domain` - the domain of type `Domain` on which the liquidity is provided (i.e., Osmosis)
/// * `owner` - the owner of the deployed program
/// * `pool_id` - the id of the GAMM liquidity pool
/// * `denom_1` - first token in the token pair of the pool
/// * `denom_2` - second token in the token pair of the pool
///
pub fn my_osmosis_gamm_program(
    osmo_domain: valence_program_manager::domain::Domain,
    owner: String,
    pool_id: u64,
    denom_1: &str,
    denom_2: &str,
) -> Result<ProgramConfig, Box<dyn Error>> {
    // Get a ProgramConfigBuilder
    let mut builder = ProgramConfigBuilder::new("osmo gamm", owner.as_str());

    // Create three accounts on Osmosis

    // An input account from which tokens are drawn to provide liquidity
    let gamm_input_acc_info = AccountInfo::new(
        "gamm_input".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    // An output account which holds the liquidity position
    let gamm_output_acc_info = AccountInfo::new(
        "gamm_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    // A final output account for the withdrawn liquidity
    let final_output_acc_info = AccountInfo::new(
        "final_output".to_string(),
        &osmo_domain,
        AccountType::default(),
    );

    // Add accounts to the program config builder
    let gamm_input_acc = builder.add_account(gamm_input_acc_info);
    let gamm_output_acc = builder.add_account(gamm_output_acc_info);
    let final_output_acc = builder.add_account(final_output_acc_info);

    // This program uses two libraries. Osmosis GAMM Liquidity Provider Library
    // and Osmosis GAMM Liquidity Withdrawer Library

    // First, the config for the GAMM LP Library is created
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

    // Second, the config for the GAMM LW Library is created
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

    // The libraries are created with their respective configs
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

    // The libraries and accounts must be registered with one another. This enables
    // the libraries to generate messages that are allowed to execute on the accounts.

    // First, establish the input_acc -> lper_lib -> output_acc link
    builder.add_link(
        &gamm_lper_library,
        vec![&gamm_input_acc],
        vec![&gamm_output_acc],
    );
    // Then, establish the output_acc -> lwer_lib -> final_output_acc link
    builder.add_link(
        &gamm_lwer_library,
        vec![&gamm_output_acc],
        vec![&final_output_acc],
    );

    // Create a function to provide liquidity
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

    // Create a function to withdraw liquidity
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

    // Create an atomic subroutine with a single function to provide liquidity
    let gamm_lper_atomic_subroutine = AtomicSubroutineBuilder::new()
        .with_function(gamm_lper_function)
        .build();
    // Authorize anyone to invoke the provide liquidity subroutine
    let gamm_lper_authorization = AuthorizationBuilder::new()
        .with_label("provide_liquidity")
        .with_subroutine(gamm_lper_atomic_subroutine)
        .build();

    // Create an atomic subroutine with a single function to withdraw liquidity
    let gamm_lwer_atomic_subroutine = AtomicSubroutineBuilder::new()
        .with_function(gamm_lwer_function)
        .build();

    // Authorize anyone to invoke the provide liquidity subroutine
    let gamm_lwer_authorization = AuthorizationBuilder::new()
        .with_label("withdraw_liquidity")
        .with_subroutine(gamm_lwer_atomic_subroutine)
        .build();

    // Add the created authorizations to the builder
    builder.add_authorization(gamm_lper_authorization);
    builder.add_authorization(gamm_lwer_authorization);

    // Build the program config and return it
    Ok(builder.build())
}
