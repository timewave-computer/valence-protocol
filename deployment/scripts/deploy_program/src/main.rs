use clap::{arg, command, Parser};
use config::Config as ConfigHelper;
use localic_utils::{NEUTRON_CHAIN_ADMIN_ADDR, NEUTRON_CHAIN_NAME};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use valence_authorization_utils::{
    authorization_message::{Message, MessageDetails, MessageType, ParamRestriction},
    builders::{AtomicFunctionBuilder, AtomicSubroutineBuilder, AuthorizationBuilder},
};
use valence_library_utils::denoms::UncheckedDenom;
use valence_program_manager::{
    account::{AccountInfo, AccountType},
    library::{LibraryConfig, LibraryInfo},
    program_config::ProgramConfig,
    program_config_builder::ProgramConfigBuilder,
};
use valence_splitter_library::msg::{UncheckedSplitAmount, UncheckedSplitConfig};

#[derive(Debug, Clone, clap::ValueEnum, Default)]
pub enum Configs {
    Testnet,
    Mainnet,
    #[default]
    Local,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enviroment config to use
    #[arg(short, long)]
    config: Configs,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    set_manager_config(args.config).await?;

    // Write your program
    let swap_amount: u128 = 1_000_000_000;

    let mut builder = ProgramConfigBuilder::new(NEUTRON_CHAIN_ADMIN_ADDR.to_string());
    let neutron_domain =
        valence_program_manager::domain::Domain::CosmosCosmwasm(NEUTRON_CHAIN_NAME.to_string());

    let account_1 = builder.add_account(AccountInfo::new(
        "test_1".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));
    let account_2 = builder.add_account(AccountInfo::new(
        "test_2".to_string(),
        &neutron_domain,
        AccountType::default(),
    ));

    let library_config = valence_splitter_library::msg::LibraryConfig {
        input_addr: account_1.clone(),
        splits: vec![UncheckedSplitConfig {
            denom: UncheckedDenom::Native("untrn".to_string()),
            account: account_2.clone(),
            amount: UncheckedSplitAmount::FixedAmount(swap_amount.into()),
        }],
    };

    let library_1 = builder.add_library(LibraryInfo::new(
        "test_splitter".to_string(),
        &neutron_domain,
        LibraryConfig::ValenceSplitterLibrary(library_config.clone()),
    ));

    builder.add_link(&library_1, vec![&account_1], vec![&account_2]);

    let action_label = "swap";
    builder.add_authorization(
        AuthorizationBuilder::new()
            .with_label(action_label)
            .with_subroutine(
                AtomicSubroutineBuilder::new()
                    .with_function(
                        AtomicFunctionBuilder::new()
                            .with_contract_address(library_1.clone())
                            .with_message_details(MessageDetails {
                                message_type: MessageType::CosmwasmExecuteMsg,
                                message: Message {
                                    name: "process_function".to_string(),
                                    params_restrictions: Some(vec![
                                        ParamRestriction::MustBeIncluded(vec![
                                            "process_function".to_string(),
                                            "split".to_string(),
                                        ]),
                                    ]),
                                },
                            })
                            .build(),
                    )
                    .build(),
            )
            .build(),
    );

    let mut program_config = builder.build();

    // Use program manager to deploy the program
    valence_program_manager::init_program(&mut program_config).await?;

    // Print instantiated program to file
    print_result(program_config)?;

    Ok(())
}

pub fn get_config(
    path: Configs,
) -> Result<valence_program_manager::config::Config, Box<dyn Error>> {
    match path {
        Configs::Testnet => ConfigHelper::builder()
            .add_source(
                glob::glob("deployment/configs/testnet/*")
                    .unwrap()
                    .filter_map(|path| {
                        let p = path.unwrap();

                        if p.is_dir() {
                            None
                        } else {
                            Some(config::File::from(p))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .add_source(
                glob::glob("deployment/configs/testnet/**/*")
                    .unwrap()
                    .filter_map(|path| {
                        let p = path.unwrap();
                        if p.is_dir() {
                            None
                        } else {
                            Some(config::File::from(p))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .build()?
            .try_deserialize()
            .map_err(|e| e.into()),
        Configs::Mainnet => ConfigHelper::builder()
            .add_source(
                glob::glob("deployment/configs/mainnet/*")
                    .unwrap()
                    .filter_map(|path| {
                        let p = path.unwrap();

                        if p.is_dir() {
                            None
                        } else {
                            Some(config::File::from(p))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .add_source(
                glob::glob("deployment/configs/mainnet/**/*")
                    .unwrap()
                    .filter_map(|path| {
                        let p = path.unwrap();
                        if p.is_dir() {
                            None
                        } else {
                            Some(config::File::from(p))
                        }
                    })
                    .collect::<Vec<_>>(),
            )
            .build()?
            .try_deserialize()
            .map_err(|e| e.into()),
        Configs::Local => ConfigHelper::builder()
            .add_source(config::File::with_name("deployment/configs/local/config"))
            .build()?
            .try_deserialize()
            .map_err(|e| e.into()),
    }
}

async fn set_manager_config(config_path: Configs) -> Result<(), Box<dyn Error>> {
    // Read the config
    let config = get_config(config_path)?;

    // Set the global config of the manager with the read config
    let mut gc = valence_program_manager::config::GLOBAL_CONFIG.lock().await;
    *gc = config;
    Ok(())
}

fn print_result(program_config: ProgramConfig) -> Result<(), Box<dyn Error>> {
    let path_name = "deployment/results";
    let path = Path::new(path_name);

    if !path.exists() {
        fs::create_dir_all(path_name)?;
    }

    // Construct the full file path
    let file_name = format!("program-{}.json", program_config.id);
    let file_path = path.join(file_name.clone());

    // Create and write to the file
    let mut file = fs::File::create(file_path.clone())?;

    // Serialize the data to a string
    let content = serde_json::to_string(&program_config)?;

    file.write_all(content.as_bytes())?;

    println!(
        "Program was instantiated successfully and written to: {}",
        file_path.display()
    );

    Ok(())
}
