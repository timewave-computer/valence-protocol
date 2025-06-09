// Purpose: Demo application showcasing Account Factory integration
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🏭 Valence Account Factory Demo");
    println!("================================");
    
    // Demo 1: Simple DeFi protocol setup
    demo_defi_protocol_setup()?;
    
    // Demo 2: Gaming platform player onboarding  
    demo_gaming_platform()?;
    
    // Demo 3: Ferry service batch processing
    demo_ferry_service()?;
    
    println!("\n✅ All demos completed successfully!");
    Ok(())
}

fn demo_defi_protocol_setup() -> Result<(), Box<dyn Error>> {
    println!("\n📊 Demo 1: DeFi Protocol Setup");
    println!("-------------------------------");
    
    // Create TokenCustody accounts for trading pools
    let pool_accounts = create_token_custody_accounts(5)?;
    println!("Created {} trading pool accounts", pool_accounts.len());
    
    // Create DataStorage account for protocol configuration
    let config_account = create_data_storage_account("protocol_config")?;
    println!("Created configuration account: {}", config_account);
    
    // Create Hybrid account for treasury management
    let treasury_account = create_hybrid_account("treasury")?;
    println!("Created treasury account: {}", treasury_account);
    
    Ok(())
}

fn demo_gaming_platform() -> Result<(), Box<dyn Error>> {
    println!("\n🎮 Demo 2: Gaming Platform");
    println!("---------------------------");
    
    // Onboard 3 players with cross-chain accounts
    for player_id in 1..=3 {
        let player_accounts = create_player_accounts(player_id)?;
        println!("Player {}: Created accounts on {} chains", 
                 player_id, player_accounts.len());
    }
    
    Ok(())
}

fn demo_ferry_service() -> Result<(), Box<dyn Error>> {
    println!("\n⛴️  Demo 3: Ferry Service Batch Processing");
    println!("-------------------------------------------");
    
    // Create batch of mixed account types
    let batch_requests = vec![
        ("user1", AccountType::TokenCustody),
        ("user2", AccountType::DataStorage),
        ("user3", AccountType::Hybrid),
        ("user4", AccountType::TokenCustody),
        ("user5", AccountType::Hybrid),
    ];
    
    let batch_results = process_account_batch(batch_requests)?;
    println!("Batch processed: {} accounts created", batch_results.len());
    
    Ok(())
}

// Helper functions (simplified for demo)
fn create_token_custody_accounts(count: usize) -> Result<Vec<String>, Box<dyn Error>> {
    let mut accounts = Vec::new();
    for i in 1..=count {
        accounts.push(format!("cosmos1pool{}address", i));
    }
    Ok(accounts)
}

fn create_data_storage_account(name: &str) -> Result<String, Box<dyn Error>> {
    Ok(format!("cosmos1{}address", name))
}

fn create_hybrid_account(name: &str) -> Result<String, Box<dyn Error>> {
    Ok(format!("cosmos1{}address", name))
}

fn create_player_accounts(player_id: u64) -> Result<Vec<String>, Box<dyn Error>> {
    // Simulate cross-chain account creation
    Ok(vec![
        format!("cosmos1player{}neutron", player_id),
        format!("0xplayer{}ethereum", player_id),
        format!("cosmos1player{}osmosis", player_id),
    ])
}

#[derive(Debug)]
enum AccountType {
    TokenCustody,
    DataStorage,
    Hybrid,
}

fn process_account_batch(requests: Vec<(&str, AccountType)>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut results = Vec::new();
    for (user, account_type) in requests {
        results.push(format!("cosmos1{}{:?}address", user, account_type));
    }
    Ok(results)
} 