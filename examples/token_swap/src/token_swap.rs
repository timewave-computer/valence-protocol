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

    program_config_builder.add_link(&library_1, vec![&account_1], vec![&account_2]);
    program_config_builder.add_link(&library_2, vec![&account_2], vec![&account_1]);

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

    let atomic_subroutine = AtomicSubroutineBuilder::new()
        .with_function(send_tokens_from_account1_to_account2)
        .with_function(send_tokens_from_account2_to_account1)
        .build();

    let authorization = AuthorizationBuilder::new()
        .with_label("atomic_swap")
        .with_mode(AuthorizationModeInfo::Permissioned(
            PermissionTypeInfo::WithoutCallLimit(vec![authorized_swap_party]),
        ))
        .with_subroutine(atomic_subroutine)
        .build();

    program_config_builder.add_authorization(authorization);

    program_config_builder.build()
}
