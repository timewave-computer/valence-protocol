use clap::{arg, command, Parser};
use config::Config as ConfigHelper;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use valence_program_manager::program_config::ProgramConfig;

#[derive(Debug, Clone, clap::ValueEnum, Default)]
pub enum Config {
    Testnet,
    Mainnet,
    #[default]
    Local,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Enviroment config to use
    #[arg(short, long, default_value = "local")]
    target_env: Config,
    /// Path to the program config file
    #[arg(short, long, default_value = "deployment/output_program/program.json")]
    program_config_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    set_manager_config(args.target_env).await?;

    // Read program config from file
    let mut program_config = get_program_config(args.program_config_path);

    // Use program manager to deploy the program
    valence_program_manager::init_program(&mut program_config).await?;

    // Print instantiated program to file
    write_result(program_config)?;

    Ok(())
}

pub fn get_program_config(path: String) -> ProgramConfig {
    let content = fs::read_to_string(path).expect("Unable to open program config file");
    serde_json::from_str::<ProgramConfig>(&content).expect("Failed to parse into ProgramConfig")
}

pub fn get_config(path: Config) -> Result<valence_program_manager::config::Config, Box<dyn Error>> {
    match path {
        Config::Testnet => ConfigHelper::builder()
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
        Config::Mainnet => ConfigHelper::builder()
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
        Config::Local => ConfigHelper::builder()
            .add_source(config::File::with_name("deployment/configs/local/config"))
            .build()?
            .try_deserialize()
            .map_err(|e| e.into()),
    }
}

async fn set_manager_config(config_path: Config) -> Result<(), Box<dyn Error>> {
    // Read the config
    let config = get_config(config_path)?;

    // Set the global config of the manager with the read config
    let mut gc = valence_program_manager::config::GLOBAL_CONFIG.lock().await;
    *gc = config;
    Ok(())
}

fn write_result(program_config: ProgramConfig) -> Result<(), Box<dyn Error>> {
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
