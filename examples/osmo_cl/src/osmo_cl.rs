use std::error::Error;

use cosmwasm_std::Int64;
use localic_utils::OSMOSIS_CHAIN_NAME;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
    domain::Domain,
};
use valence_osmosis_utils::utils::cl_utils::TickRange;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};

/// Returns a `ProgramConfig` for a program that provides liquidity to a concentrated
/// liquidity pool on Osmosis. The position is held in a program controlled account until
/// it is withdrawn.
///
/// The returned `ProgramConfig` can be used to deploy and configure the program.
/// The accompanying osmo_cl_test.rs demonstrates local deployment and testing of
/// this program.
///
/// # Arguments
/// * `domain` - the domain of type `Domain` on which the liquidity is provided (i.e., Osmosis)
/// * `owner` - the owner of the deployed program
/// * `pool_id` - the id of the concentrated liquidity pool
/// * `denom_1` - first token in the token pair of the pool
/// * `denom_2` - second token in the token pair of the pool
/// * `lower_tick` - liquidity is provided between two ticks: lower and upper
/// * `upper_tick` - liquidity is provided between two ticks: lower and upper
///   
pub fn my_osmosis_cl_program(
    osmo_domain: valence_program_manager::domain::Domain,
    owner: String,
    pool_id: u64,
    denom_1: &str,
    denom_2: &str,
    lower_tick: Int64,
    upper_tick: Int64,
) -> Result<ProgramConfig, Box<dyn Error>> {
    // initialize program config builder
    let mut builder = ProgramConfigBuilder::new("osmo cl", owner.as_str());

    // Create three accounts on Osmosis

    // An input account from which tokens are drawn to provide liquidity
    let cl_input_acc_info =
        AccountInfo::new("cl_input".to_string(), &osmo_domain, AccountType::default());

    // An output account which holds the liquidity position
    let cl_output_acc_info = AccountInfo::new(
        "cl_output".to_string(),
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
    let cl_input_acc = builder.add_account(cl_input_acc_info);
    let cl_output_acc = builder.add_account(cl_output_acc_info);
    let final_output_acc = builder.add_account(final_output_acc_info);

    // This program uses two libraries. Osmosis Liquidity Provider and
    // Osmosis Liquidity Withdrawer.

    // First, the Osmosis Concentrated Liquidity Provider config struct is
    // created
    let cl_lper_config = valence_osmosis_cl_lper::msg::LibraryConfig {
        input_addr: cl_input_acc.clone(),
        output_addr: cl_output_acc.clone(),
        lp_config: valence_osmosis_cl_lper::msg::LiquidityProviderConfig {
            pool_id: pool_id.into(),
            pool_asset_1: denom_1.to_string(),
            pool_asset_2: denom_2.to_string(),
            global_tick_range: TickRange {
                lower_tick,
                upper_tick,
            },
        },
    };

    // Second, the Osmosis Concentrated Liquidity Withdrawer config struct is
    // created
    let cl_withdrawer_config = valence_osmosis_cl_withdrawer::msg::LibraryConfig {
        input_addr: cl_output_acc.clone(),
        output_addr: final_output_acc.clone(),
        pool_id: pool_id.into(),
    };

    // The libraries are created with their respective configs
    let cl_lper_library_info = LibraryInfo::new(
        "test_cl_lper".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisClLper(cl_lper_config),
    );
    let cl_withdrawer_library_info = LibraryInfo::new(
        "test_cl_withdrawer".to_string(),
        &osmo_domain,
        LibraryConfig::ValenceOsmosisClWithdrawer(cl_withdrawer_config),
    );
    let cl_lper_library = builder.add_library(cl_lper_library_info);
    let cl_withdrawer_library = builder.add_library(cl_withdrawer_library_info);

    // The libraries and accounts must be registered with one another. This enables
    // the libraries to generate messages that are allowed to execute on the accounts.

    // First, establish the input_acc -> lper_lib -> output_acc link
    builder.add_link(&cl_lper_library, vec![&cl_input_acc], vec![&cl_output_acc]);
    // Then, establish the output_acc -> lwer_lib -> final_output_acc link
    builder.add_link(
        &cl_withdrawer_library,
        vec![&cl_output_acc],
        vec![&final_output_acc],
    );

    // Create a function to provide liquidity
    let cl_lper_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(cl_lper_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    // Create a function to withdraw liquidity
    let cl_withdrawer_function = AtomicFunctionBuilder::new()
        .with_domain(Domain::External(OSMOSIS_CHAIN_NAME.to_string()))
        .with_contract_address(cl_withdrawer_library.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: None,
            },
        })
        .build();

    // Create an atomic subroutine with a single function to provide liquidity
    let cl_lper_subroutine = AtomicSubroutineBuilder::new()
        .with_function(cl_lper_function)
        .build();
    // Authorize anyone to invoke the provide liquidity subroutine
    let cl_lper_authorization = AuthorizationBuilder::new()
        .with_label("provide_liquidity")
        .with_subroutine(cl_lper_subroutine)
        .build();

    // Create an atomic subroutine with a single function to withdraw liquidity
    let cl_withdrawer_subroutine = AtomicSubroutineBuilder::new()
        .with_function(cl_withdrawer_function)
        .build();
    // Authorize anyone to invoke the provide liquidity subroutine
    let cl_withdrawer_authorization = AuthorizationBuilder::new()
        .with_label("withdraw_liquidity")
        .with_subroutine(cl_withdrawer_subroutine)
        .build();

    // Add the created authorizations to the builder
    builder.add_authorization(cl_lper_authorization);
    builder.add_authorization(cl_withdrawer_authorization);

    // Build the program config and return it
    Ok(builder.build())
}
