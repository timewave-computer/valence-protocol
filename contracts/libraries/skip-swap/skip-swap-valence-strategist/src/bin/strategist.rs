use cosmwasm_std::Addr;
use skip_swap_valence_strategist::{
    chain::ChainClient,
    orchestrator::{Orchestrator, OrchestratorConfig},
    skipapi::SkipApi,
    config::{load_config, StrategistConfig},
    strategist::Strategist,
};
use std::collections::HashMap;
use std::{env, fs};
use std::path::Path;
use std::process;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    library_address: String,
    skip_entry_point: String,
    skip_api_url: String,
    skip_api_key: Option<String>,
    polling_interval: u64,
    max_retries: u8,
    retry_delay: u64,
    strategist_address: String,
    monitored_accounts: HashMap<String, String>,
}

#[cfg(feature = "runtime")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());
    
    println!("Loading configuration from {}", config_path);
    
    if !Path::new(&config_path).exists() {
        eprintln!("Configuration file not found: {}", config_path);
        process::exit(1);
    }
    
    let config_content = fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&config_content)?;
    
    // Convert the string addresses to Addr
    let library_address = Addr::unchecked(config.library_address);
    let strategist_address = Addr::unchecked(config.strategist_address);
    
    let mut monitored_accounts = HashMap::new();
    for (token, address) in config.monitored_accounts {
        monitored_accounts.insert(token, Addr::unchecked(address));
    }
    
    // Create orchestrator config
    let orchestrator_config = OrchestratorConfig {
        library_address,
        monitored_accounts,
        polling_interval: config.polling_interval,
        max_retries: config.max_retries,
        retry_delay: config.retry_delay,
        skip_api_url: config.skip_api_url.clone(),
    };
    
    println!("Initializing strategist with address {}", strategist_address);
    
    // Create chain client
    let chain_client = ChainClient::new(strategist_address);
    
    // Create Skip API client
    let skip_api = SkipApi::new(&config.skip_api_url, config.skip_api_key);
    
    // Create orchestrator
    let mut orchestrator = Orchestrator::new(chain_client, skip_api, orchestrator_config);
    
    println!("Starting polling loop...");
    
    // Start polling
    orchestrator.start_polling().unwrap_or_else(|e| {
        eprintln!("Error in polling loop: {}", e);
        process::exit(1);
    });
    
    Ok(())
}

// Fallback main function for when the runtime feature is not enabled
#[cfg(not(feature = "runtime"))]
fn main() {
    eprintln!("Please enable the 'runtime' feature to run the strategist binary");
    process::exit(1);
} 