use valence_authorization_utils::{
    authorization::{AuthorizationModeInfo, PermissionTypeInfo},
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_library_utils::denoms::UncheckedDenom;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    domain::Domain,
    library::{LibraryConfig, LibraryInfo},
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};
use valence_splitter_library::msg::{UncheckedSplitAmount, UncheckedSplitConfig};

/// Returns a `ProgramConfig` for a program that atomically swap tokens 
/// between two program controlled accounts on a given domain. 
/// This `ProgramConfig` can be used to deploy and configure the program. 
/// The accompanying token_swap_test.rs demonstrates local deployment and testing of 
/// this program.
///
/// # Arguments
/// * `domain` - the domain of type `Domain` on which the swap occurs (e.g Neutron)
/// * `owner` - the owner of the deployed program
/// * `token1` - the token denomination required in the first account
/// * `token2` - the token denomination required in the second account
/// * `swap_amount_token1` - the amount of the first token to swap
/// * `swap_amount_token2` - the amount of the second to to swap
/// * `authorized_swap_party` - the party that receives the authority token to execute
///                             program
pub fn my_atomic_token_swap_program(
    domain: Domain,
    owner: String,
    token1: String,
    token2: String,
    swap_amount_token1: u128,
    swap_amount_token2: u128,
    authorized_swap_party: String,
) -> ProgramConfig {
    let mut program_config_builder = ProgramConfigBuilder::new(owner);

    // Create two accounts in the domain
    let account_1 = program_config_builder.add_account(AccountInfo::new(
        "base_account_1".to_string(),
        &domain,
        AccountType::default(),
    ));

    let account_2 = program_config_builder.add_account(AccountInfo::new(
        "base_account_2".to_string(),
        &domain,
        AccountType::default(),
    ));

    // Valence provides a splitter library that can send tokens from 1 accounts to N other
    // accounts. In the atomic swap example, two instances of the library are configured.

    // The first splitter instance is configured to send a fixed `swap_amount_token1` from 
    // the first account to the second account.
    let library_1 = program_config_builder.add_library(LibraryInfo::new(
        "splitter_1".to_string(),
        &domain,
        LibraryConfig::ValenceSplitterLibrary(valence_splitter_library::msg::LibraryConfig {
            input_addr: account_1.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token1.to_string()),
                account: account_2.clone(),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount_token1.into()),
            }],
        }),
    ));
    // The second splitter instance is configured to send a fixed `swap_amount_token2`
    // from the second account to the the first account.
    let library_2 = program_config_builder.add_library(LibraryInfo::new(
        "splitter_2".to_string(),
        &domain,
        LibraryConfig::ValenceSplitterLibrary(valence_splitter_library::msg::LibraryConfig {
            input_addr: account_2.clone(),
            splits: vec![UncheckedSplitConfig {
                denom: UncheckedDenom::Native(token2.to_string()),
                account: account_1.clone(),
                amount: UncheckedSplitAmount::FixedAmount(swap_amount_token2.into()),
            }],
        }),
    ));

    // The two splitter instances are registered with the two accounts.
    program_config_builder.add_link(&library_1, vec![&account_1], vec![&account_2]);
    program_config_builder.add_link(&library_2, vec![&account_2], vec![&account_1]);

    // A function is built to call the split function on the first splitter.
    let send_tokens_from_account1_to_account2 = AtomicFunctionBuilder::new()
        .with_contract_address(library_1.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "split".to_string(),
                ])]),
            },
        })
        .build();

    // A function is built do call the split function on the second splitter.
    let send_tokens_from_account2_to_account1 = AtomicFunctionBuilder::new()
        .with_contract_address(library_2.clone())
        .with_message_details(MessageDetails {
            message_type: MessageType::CosmwasmExecuteMsg,
            message: Message {
                name: "process_function".to_string(),
                params_restrictions: Some(vec![ParamRestriction::MustBeIncluded(vec![
                    "process_function".to_string(),
                    "split".to_string(),
                ])]),
            },
        })
        .build();

    // To make this swap occur atomically, the two functions are combined into
    // an atomic subroutine.
    let atomic_subroutine = AtomicSubroutineBuilder::new()
        .with_function(send_tokens_from_account1_to_account2)
        .with_function(send_tokens_from_account2_to_account1)
        .build();

    // An authorization is generated that will create an authority token `atomic_swap`
    // and give the authority token to the `authorized_swap_party`. This party
    // is given the ability to invoke the `atomic_subroutine` created previously.
    // The `PermissionTypeInfo::WithoutCallLimit` signifies that the party can invoke
    // the subroutine an indefinite number of times.  
    let authorization = AuthorizationBuilder::new()
        .with_label("atomic_swap")
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![authorized_swap_party]),
        ))
        .with_subroutine(atomic_subroutine)
        .build();

    program_config_builder.add_authorization(authorization);
    
    // The program config is built.
    program_config_builder.build()
}
