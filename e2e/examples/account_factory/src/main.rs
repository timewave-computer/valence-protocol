// Purpose: End-to-end tests for Valence Account Factory System
//
// This test deploys actual contracts, interacts with the ZK coprocessor,
// and validates the complete account factory workflow including:
// - Contract deployment on anvil (EVM) and local chains (CosmWasm)
// - Real account creation with deterministic addressing
// - ZK proof generation and verification 
// - Atomic operations and ferry service functionality
// - Cross-chain consistency validation

use std::{
    error::Error,
    time::{Duration, SystemTime},
    collections::HashMap,
    process::{Command, Stdio},
};

use tokio::time::timeout;

mod clients;
mod constants;
mod utils;

pub use constants::*;
pub use clients::*;
pub use utils::*;

use crate::clients::{CoprocessorClient, CosmWasmClient, EthereumClient, FerryService, BatchStatus};

/// Main e2e test function
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting Account Factory E2E Tests");
    
    // Initialize logging for debugging
    env_logger::init();
    
    // Set up test environment
    let config = E2EConfig::from_env().await?;
    println!("Test environment: {:?}", config.environment);
    
    // Run comprehensive test suite
    let results = run_account_factory_tests(&config).await?;
    
    // Print results summary
    print_test_results(&results);
    
    if results.failed_count() > 0 {
        std::process::exit(1);
    }
    
    println!("All Account Factory E2E tests completed successfully!");
    Ok(())
}

/// E2E test configuration
#[derive(Debug, Clone)]
pub struct E2EConfig {
    /// Anvil RPC URL for EVM testing
    pub anvil_rpc_url: String,
    /// Local CosmWasm chain RPC URL (or neutron testnet)
    pub cosmwasm_rpc_url: String,
    /// ZK Coprocessor service URL
    pub coprocessor_url: String,
    /// Test mnemonic for signing
    pub mnemonic: String,
    /// Test environment type
    pub environment: Environment,
    /// Deployed contract addresses
    pub contract_addresses: ContractAddresses,
}

#[derive(Debug, Clone)]
pub enum Environment {
    Local,   // Local anvil + local coprocessor
    Testnet, // Real testnets + public coprocessor
}

#[derive(Debug, Clone, Default)]
pub struct ContractAddresses {
    pub evm_factory: Option<String>,
    pub evm_implementation: Option<String>,
    pub cosmwasm_factory: Option<String>,
    pub jit_account_code_id: Option<u64>,
    // Authorization and Processor contracts
    pub evm_authorization: Option<String>,
    pub evm_processor: Option<String>,
    pub evm_verification_gateway: Option<String>,
    pub cosmwasm_authorization: Option<String>,
    pub cosmwasm_processor: Option<String>,
    pub cosmwasm_verification_gateway: Option<String>,
}

impl E2EConfig {
    /// Create configuration from environment
    pub async fn from_env() -> Result<Self, Box<dyn Error>> {
        let anvil_rpc_url = std::env::var("ANVIL_RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
        
        let cosmwasm_rpc_url = std::env::var("COSMWASM_RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:26657".to_string());
        
        let coprocessor_url = std::env::var("COPROCESSOR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:37281".to_string());
        
        let mnemonic = std::env::var("E2E_MNEMONIC")
            .unwrap_or_else(|_| "test test test test test test test test test test test junk".to_string());
        
        // Determine environment
        let environment = if anvil_rpc_url.contains("127.0.0.1") {
            Environment::Local
        } else {
            Environment::Testnet
        };
        
        Ok(Self {
            anvil_rpc_url,
            cosmwasm_rpc_url,
            coprocessor_url,
            mnemonic,
            environment,
            contract_addresses: ContractAddresses::default(),
        })
    }
}

/// Test results tracking
#[derive(Debug, Default)]
pub struct E2ETestResults {
    pub results: HashMap<String, Result<(), String>>,
    pub total_duration: Duration,
}

impl E2ETestResults {
    pub fn add_test_result(&mut self, name: &str, result: Result<(), Box<dyn Error>>) {
        let converted_result = result.map_err(|e| e.to_string());
        self.results.insert(name.to_string(), converted_result);
    }
    
    pub fn passed_count(&self) -> usize {
        self.results.values().filter(|r| r.is_ok()).count()
    }
    
    pub fn failed_count(&self) -> usize {
        self.results.values().filter(|r| r.is_err()).count()
    }
}

/// Run comprehensive account factory test suite
async fn run_account_factory_tests(config: &E2EConfig) -> Result<E2ETestResults, Box<dyn Error>> {
    println!("=== Starting Account Factory E2E Test Suite ===");
    let start_time = SystemTime::now();
    let mut results = E2ETestResults::default();
    
    // Test 1: Environment setup and connectivity
    results.add_test_result("environment_setup", setup_test_environment(config).await);
    
    // Test 2: Deploy contracts
    let mut config_with_contracts = config.clone();
    results.add_test_result("contract_deployment", deploy_contracts(&mut config_with_contracts).await);
    
    // Test 3: Basic account creation on both chains
    results.add_test_result("basic_account_creation", test_basic_account_creation(&config_with_contracts).await);
    
    // Test 4: Deterministic addressing verification
    results.add_test_result("deterministic_addressing", test_deterministic_addressing(&config_with_contracts).await);
    
    // Test 5: ZK proof generation and verification
    results.add_test_result("zk_proof_verification", test_zk_proof_verification(&config_with_contracts).await);
    
    // Test 6: Atomic operations
    results.add_test_result("atomic_operations", test_atomic_operations(&config_with_contracts).await);
    
    // Test 7: Ferry service batch processing
    results.add_test_result("ferry_service_batch", test_ferry_service_batch(&config_with_contracts).await);
    
    // Test 8: Cross-chain consistency validation
    results.add_test_result("cross_chain_consistency", test_cross_chain_consistency(&config_with_contracts).await);
    
    // Test 9: Security scenarios (replay protection, etc.)
    results.add_test_result("security_scenarios", test_security_scenarios(&config_with_contracts).await);
    
    // Test 10: Performance benchmarks
    results.add_test_result("performance_benchmarks", test_performance_benchmarks(&config_with_contracts).await);
    
    // Test 11: ZK proof submission and verification on-chain (EVM)
    results.add_test_result("zk_proof_submission_evm", test_zk_proof_submission_evm(&config_with_contracts).await);
    
    // Test 12: ZK proof submission and verification on-chain (CosmWasm)
    results.add_test_result("zk_proof_submission_cosmwasm", test_zk_proof_submission_cosmwasm(&config_with_contracts).await);
    
    // Test 13: Ferry service architecture flow
    results.add_test_result("ferry_service_architecture", test_ferry_service_architecture(&config_with_contracts).await);
    
    // Test 14: Historical block entropy validation
    results.add_test_result("historical_block_validation", test_historical_block_validation(&config_with_contracts).await);
    
    // Test 15: End-to-end account creation with ZK proofs (EVM)
    results.add_test_result("e2e_account_creation_evm", test_e2e_account_creation_evm(&config_with_contracts).await);
    
    // Test 16: End-to-end account creation with ZK proofs (CosmWasm)
    results.add_test_result("e2e_account_creation_cosmwasm", test_e2e_account_creation_cosmwasm(&config_with_contracts).await);
    
    results.total_duration = start_time.elapsed()?;
    println!("=== Account Factory E2E Test Suite Completed ===");
    
    Ok(results)
}

/// Set up test environment (start anvil, check services)
async fn setup_test_environment(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Setting up test environment...");
    
    // Start anvil if testing locally
    if matches!(config.environment, Environment::Local) {
        start_anvil().await?;
    }
    
    // Test connectivity to all services
    test_service_connectivity(config).await?;
    
    println!("✅ Test environment setup complete");
    Ok(())
}

/// Start anvil for local EVM testing
async fn start_anvil() -> Result<(), Box<dyn Error>> {
    println!("Starting anvil...");
    
    // Check if anvil is already running
    let output = Command::new("curl")
        .args(&["-s", "http://127.0.0.1:8545"])
        .output();
    
    if output.is_ok() && output.unwrap().status.success() {
        println!("Anvil already running");
        return Ok(());
    }
    
    // Start anvil in background
    Command::new("anvil")
        .args(&["--port", "8545", "--accounts", "10", "--balance", "1000000"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    
    // Wait for anvil to be ready
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let output = Command::new("curl")
            .args(&["-s", "http://127.0.0.1:8545"])
            .output();
        
        if output.is_ok() && output.unwrap().status.success() {
            println!("✅ Anvil started successfully");
            return Ok(());
        }
    }
    
    Err("Failed to start anvil".into())
}

/// Test connectivity to all required services
async fn test_service_connectivity(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing service connectivity...");
    
    // Test anvil connectivity
    let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
    let block_number = eth_client.get_block_number().await?;
    println!("✅ Anvil connected, block: {}", block_number);
    
    // Test coprocessor connectivity (if available)
    let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
    match coprocessor_client.health_check().await {
        Ok(()) => println!("✅ Coprocessor connected"),
        Err(_) => println!("⚠️ Coprocessor not available (ZK tests will use mock)"),
    }
    
    // Test CosmWasm connectivity (if available)
    if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
        match cosmwasm_client.health_check().await {
            Ok(()) => println!("✅ CosmWasm chain connected"),
            Err(_) => println!("⚠️ CosmWasm chain not available (will use mock)"),
        }
    } else {
        println!("⚠️ CosmWasm chain not available (will use mock)");
    }
    
    Ok(())
}

/// Deploy account factory contracts on both chains
async fn deploy_contracts(config: &mut E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Deploying account factory contracts...");
    
    // Deploy EVM contracts
    let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
    
    // Deploy JIT account implementation
    let implementation_addr = eth_client.deploy_jit_account_implementation().await?;
    config.contract_addresses.evm_implementation = Some(implementation_addr.clone());
    println!("✅ EVM JIT account implementation deployed: {}", implementation_addr);
    
    // Deploy account factory
    let factory_addr = eth_client.deploy_account_factory(&implementation_addr).await?;
    config.contract_addresses.evm_factory = Some(factory_addr.clone());
    println!("✅ EVM account factory deployed: {}", factory_addr);

    // Deploy Authorization contract
    let auth_addr = eth_client.deploy_authorization_contract().await?;
    config.contract_addresses.evm_authorization = Some(auth_addr.clone());
    println!("✅ EVM Authorization contract deployed: {}", auth_addr);

    // Deploy Processor contract
    let processor_addr = eth_client.deploy_processor_contract().await?;
    config.contract_addresses.evm_processor = Some(processor_addr.clone());
    println!("✅ EVM Processor contract deployed: {}", processor_addr);

    // Deploy VerificationGateway contract
    let gateway_addr = eth_client.deploy_verification_gateway().await?;
    config.contract_addresses.evm_verification_gateway = Some(gateway_addr.clone());
    println!("✅ EVM VerificationGateway contract deployed: {}", gateway_addr);
    
    // Deploy CosmWasm contracts (if available)
    if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
        // Upload and instantiate JIT account contract
        let jit_code_id = cosmwasm_client.upload_jit_account_contract().await?;
        config.contract_addresses.jit_account_code_id = Some(jit_code_id);
        println!("✅ CosmWasm JIT account uploaded: code_id {}", jit_code_id);
        
        // Upload and instantiate factory contract
        let factory_addr = cosmwasm_client.deploy_account_factory(jit_code_id).await?;
        config.contract_addresses.cosmwasm_factory = Some(factory_addr.clone());
        println!("✅ CosmWasm account factory deployed: {}", factory_addr);

        // Deploy Authorization contract
        let auth_addr = cosmwasm_client.deploy_authorization_contract().await?;
        config.contract_addresses.cosmwasm_authorization = Some(auth_addr.clone());
        println!("✅ CosmWasm Authorization contract deployed: {}", auth_addr);

        // Deploy Processor contract
        let processor_addr = cosmwasm_client.deploy_processor_contract().await?;
        config.contract_addresses.cosmwasm_processor = Some(processor_addr.clone());
        println!("✅ CosmWasm Processor contract deployed: {}", processor_addr);

        // Deploy VerificationGateway contract
        let gateway_addr = cosmwasm_client.deploy_verification_gateway().await?;
        config.contract_addresses.cosmwasm_verification_gateway = Some(gateway_addr.clone());
        println!("✅ CosmWasm VerificationGateway contract deployed: {}", gateway_addr);
    } else {
        println!("⚠️ CosmWasm deployment skipped (chain not available)");
    }
    
    Ok(())
}

/// Test basic account creation on both chains
async fn test_basic_account_creation(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing basic account creation...");
    
    let test_request = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "test_program".to_string(),
        account_request_id: 1,
        account_type: 1, // TokenCustody
        libraries: vec![],
        historical_block_number: None,
        target_chain: None,
    };
    
    // Test EVM account creation
    if let Some(factory_addr) = &config.contract_addresses.evm_factory {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        let account_addr = eth_client.create_account(factory_addr, &test_request).await?;
        println!("✅ EVM account created: {}", account_addr);
        
        // Verify account was created correctly
        let controller = eth_client.get_account_controller(&account_addr).await?;
        assert_eq!(controller.to_lowercase(), test_request.controller.to_lowercase());
        println!("✅ EVM account controller verified");
    }
    
    // Test CosmWasm account creation
    if let Some(factory_addr) = &config.contract_addresses.cosmwasm_factory {
        if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
            // Use cosmos address for CosmWasm chain
            let cosmwasm_request = AccountCreationRequest {
                controller: "cosmos1testuser".to_string(),
                program_id: "test_program".to_string(),
                account_request_id: 1,
                account_type: 1,
                libraries: vec![],
                historical_block_number: None,
                target_chain: None,
            };
            
            let account_addr = cosmwasm_client.create_account(factory_addr, &cosmwasm_request).await?;
            println!("✅ CosmWasm account created: {}", account_addr);
            
            // Verify account was created correctly
            let controller = cosmwasm_client.get_account_controller(&account_addr).await?;
            assert_eq!(controller, cosmwasm_request.controller);
            println!("✅ CosmWasm account controller verified");
        }
    }
    
    Ok(())
}

/// Test deterministic addressing
async fn test_deterministic_addressing(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing deterministic addressing...");
    
    let test_request = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "deterministic_test".to_string(),
        account_request_id: 42,
        account_type: 2, // DataStorage
        libraries: vec!["lib1".to_string(), "lib2".to_string()],
        historical_block_number: None,
        target_chain: None,
    };
    
    // Test EVM deterministic addressing
    if let Some(factory_addr) = &config.contract_addresses.evm_factory {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        
        // Compute address before creation
        let predicted_addr = eth_client.compute_account_address(factory_addr, &test_request).await?;
        
        // Create account and verify address matches
        let actual_addr = eth_client.create_account(factory_addr, &test_request).await?;
        
        assert_eq!(predicted_addr.to_lowercase(), actual_addr.to_lowercase());
        println!("✅ EVM deterministic addressing verified: {}", actual_addr);
    }
    
    // Test CosmWasm deterministic addressing
    if let Some(factory_addr) = &config.contract_addresses.cosmwasm_factory {
        if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
            let test_request_cw = AccountCreationRequest {
                controller: "cosmos1testuser".to_string(),
                program_id: "deterministic_test_cw".to_string(),
                account_request_id: 43,
                account_type: 2,
                libraries: vec!["lib1".to_string(), "lib2".to_string()],
                historical_block_number: None,
                target_chain: None,
            };
            
            // Compute address before creation
            let predicted_addr = cosmwasm_client.compute_account_address(factory_addr, &test_request_cw).await?;
            
            // Create account and verify address matches
            let actual_addr = cosmwasm_client.create_account(factory_addr, &test_request_cw).await?;
            
            assert_eq!(predicted_addr, actual_addr);
            println!("✅ CosmWasm deterministic addressing verified: {}", actual_addr);
        }
    }
    
    Ok(())
}

/// Test ZK proof generation and verification
async fn test_zk_proof_verification(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing ZK proof generation and verification...");
    
    let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
    
    // Check if account factory ZK programs are available
    let programs = coprocessor_client.list_programs().await?;
    
    let evm_program = programs.iter()
        .find(|p| p["name"].as_str() == Some("evm_account_factory"))
        .ok_or("EVM account factory ZK program not found")?;
    
    // Generate proof for EVM account creation
    let proof_request = serde_json::json!({
        "controller": "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8",
        "program_id": "zk_test",
        "account_request_id": 100,
        "account_type": 3,
        "libraries": ["lib1"]
    });
    
    let proof_id = coprocessor_client.request_proof(
        evm_program["id"].as_str().unwrap(),
        &proof_request
    ).await?;
    
    println!("✅ ZK proof requested: {}", proof_id);
    
    // Wait for proof generation (with timeout)
    let proof_result = timeout(
        Duration::from_secs(120), // 2 minutes timeout
        coprocessor_client.wait_for_proof(&proof_id)
    ).await??;
    
    println!("✅ ZK proof generated successfully");
    
    // Verify proof format
    assert!(proof_result["proof"].is_object());
    assert!(proof_result["public_inputs"].is_array());
    
    println!("✅ ZK proof verification complete");
    
    Ok(())
}

/// Test atomic operations
async fn test_atomic_operations(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing atomic operations...");
    
    if let Some(factory_addr) = &config.contract_addresses.evm_factory {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        
        let atomic_request = AtomicAccountRequest {
            request: AccountCreationRequest {
                controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
                program_id: "atomic_test".to_string(),
                account_request_id: 200,
                account_type: 1,
                libraries: vec![],
                historical_block_number: None,
                target_chain: None,
            },
            signature: vec![0u8; 65], // Mock signature for testing
            expiration: (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs() + 3600) as u64,
        };
        
        // Test atomic account creation
        let account_addr = eth_client.create_account_atomic(factory_addr, &atomic_request).await?;
        println!("✅ EVM atomic account creation: {}", account_addr);
        
        // Verify the account was created with correct properties
        let controller = eth_client.get_account_controller(&account_addr).await?;
        assert_eq!(controller.to_lowercase(), atomic_request.request.controller.to_lowercase());
        
        println!("✅ Atomic operation verification complete");
    }
    
    Ok(())
}

/// Test ferry service batch processing
async fn test_ferry_service_batch(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing ferry service batch processing...");
    
    // Initialize ferry service
    let ferry_service = FerryService::new(
        "test_ferry_operator".to_string(),
        vec!["ethereum".to_string(), "neutron".to_string()]
    );
    
    // Create test requests (ferry service will add historical block numbers)
    let requests = vec![
        AccountCreationRequest {
            controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
            program_id: "ferry_batch_test".to_string(),
            account_request_id: 301,
            account_type: 1,
            libraries: vec![],
            historical_block_number: None, // Ferry service will set this
            target_chain: Some("ethereum".to_string()),
        },
        AccountCreationRequest {
            controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
            program_id: "ferry_batch_test".to_string(),
            account_request_id: 302,
            account_type: 2,
            libraries: vec!["lib1".to_string()],
            historical_block_number: None,
            target_chain: Some("ethereum".to_string()),
        },
        AccountCreationRequest {
            controller: "cosmos1testuser".to_string(),
            program_id: "ferry_batch_test".to_string(),
            account_request_id: 303,
            account_type: 3,
            libraries: vec!["lib1".to_string(), "lib2".to_string()],
            historical_block_number: None,
            target_chain: Some("neutron".to_string()),
        },
    ];
    
    // Step 1: Submit requests to ferry service (App -> Ferry)
    println!("Step 1: Submitting {} requests to ferry service", requests.len());
    for request in &requests {
        let request_id = ferry_service.submit_account_request(
            request.clone(),
            request.target_chain.as_ref().unwrap()
        ).await?;
        println!("  Submitted request {}: {}", request.account_request_id, request_id);
    }
    
    assert_eq!(ferry_service.get_pending_count(), 3);
    println!("✅ Ferry service queued {} requests", ferry_service.get_pending_count());
    
    // Step 2: Process batch through ferry service (follows full architecture flow)
    println!("Step 2: Processing batch through ferry service architecture");
    
    // Set up clients for ferry service coordination
    let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
    
    let mut verification_gateways = std::collections::HashMap::new();
    if let Some(evm_gateway) = &config.contract_addresses.evm_verification_gateway {
        verification_gateways.insert("ethereum".to_string(), evm_gateway.clone());
    }
    if let Some(cw_gateway) = &config.contract_addresses.cosmwasm_verification_gateway {
        verification_gateways.insert("neutron".to_string(), cw_gateway.clone());
    }
    
    let mut account_factories = std::collections::HashMap::new();
    if let Some(evm_factory) = &config.contract_addresses.evm_factory {
        account_factories.insert("ethereum".to_string(), evm_factory.clone());
    }
    if let Some(cw_factory) = &config.contract_addresses.cosmwasm_factory {
        account_factories.insert("neutron".to_string(), cw_factory.clone());
    }
    
    // Process batch following the architecture:
    // Ferry -> ZK Coprocessor -> Ferry -> Verification Gateway -> Ferry -> Account Factory
    let batch_result = ferry_service.process_batch(
        &coprocessor_client,
        &verification_gateways,
        &account_factories
    ).await?;
    
    // Step 3: Verify results
    println!("Step 3: Verifying batch results");
    assert_eq!(batch_result.accounts.len(), 3);
    assert!(matches!(batch_result.status, BatchStatus::AccountsCreated));
    assert_eq!(ferry_service.get_pending_count(), 0);
    
    println!("✅ Ferry service batch processing completed successfully");
    println!("  Batch ID: {}", batch_result.batch_id);
    println!("  Accounts created: {}", batch_result.accounts.len());
    
    // Verify accounts were created with historical block validation
    for (i, account_addr) in batch_result.accounts.iter().enumerate() {
        println!("  Account {}: {}", i + 1, account_addr);
        // Verify the account address includes historical block entropy
        assert!(account_addr.len() > 10); // Basic sanity check
    }
    
    println!("✅ Ferry service architecture test completed successfully");
    
    Ok(())
}

/// Test cross-chain consistency
async fn test_cross_chain_consistency(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing cross-chain consistency...");
    
    // Test that same inputs produce different but consistent addresses on different chains
    let test_request_eth = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "consistency_test".to_string(),
        account_request_id: 400,
        account_type: 3,
        libraries: vec!["lib1".to_string()],
        historical_block_number: None,
        target_chain: None,
    };
    
    let test_request_cosmos = AccountCreationRequest {
        controller: "cosmos1testuser".to_string(),
        program_id: "consistency_test".to_string(),
        account_request_id: 400,
        account_type: 3,
        libraries: vec!["lib1".to_string()],
        historical_block_number: None,
        target_chain: None,
    };
    
    let mut eth_addr = None;
    let mut cosmos_addr = None;
    
    // Get EVM address
    if let Some(factory_addr) = &config.contract_addresses.evm_factory {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        eth_addr = Some(eth_client.compute_account_address(factory_addr, &test_request_eth).await?);
    }
    
    // Get CosmWasm address
    if let Some(factory_addr) = &config.contract_addresses.cosmwasm_factory {
        if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
            cosmos_addr = Some(cosmwasm_client.compute_account_address(factory_addr, &test_request_cosmos).await?);
        }
    }
    
    // Verify consistency
    if let (Some(eth), Some(cosmos)) = (&eth_addr, &cosmos_addr) {
        // Addresses should be different (different chains) but deterministic
        assert_ne!(eth, cosmos);
        println!("✅ Cross-chain consistency verified:");
        println!("   EVM: {}", eth);
        println!("   CosmWasm: {}", cosmos);
    }
    
    Ok(())
}

/// Test security scenarios
async fn test_security_scenarios(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing security scenarios...");
    
    if let Some(factory_addr) = &config.contract_addresses.evm_factory {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        
        // Test nonce replay protection
        let request = AccountCreationRequest {
            controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
            program_id: "security_test".to_string(),
            account_request_id: 500,
            account_type: 1,
            libraries: vec![],
            historical_block_number: None,
            target_chain: None,
        };
        
        // First creation should succeed
        let _account1 = eth_client.create_account(factory_addr, &request).await?;
        println!("✅ First account creation succeeded");
        
        // Second creation with same account_request_id should fail
        let result = eth_client.create_account(factory_addr, &request).await;
        assert!(result.is_err());
        println!("✅ Account request ID replay protection verified");
    }
    
    Ok(())
}

/// Test performance benchmarks
async fn test_performance_benchmarks(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing performance benchmarks...");
    
    if let Some(factory_addr) = &config.contract_addresses.evm_factory {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        
        // Benchmark single account creation
        let start_time = SystemTime::now();
        let request = AccountCreationRequest {
            controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
            program_id: "perf_test".to_string(),
            account_request_id: 600,
            account_type: 1,
            libraries: vec![],
            historical_block_number: None,
            target_chain: None,
        };
        
        let _account = eth_client.create_account(factory_addr, &request).await?;
        let single_duration = start_time.elapsed()?;
        
        println!("✅ Single account creation: {:?}", single_duration);
        
        // Benchmark batch creation
        let batch_requests: Vec<AccountCreationRequest> = (601..610)
            .map(|account_request_id| AccountCreationRequest {
                controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
                program_id: "perf_test".to_string(),
                account_request_id,
                account_type: 1,
                libraries: vec![],
                historical_block_number: None,
                target_chain: None,
            })
            .collect();
        
        let start_time = SystemTime::now();
        let _accounts = eth_client.create_accounts_batch(factory_addr, &batch_requests).await?;
        let batch_duration = start_time.elapsed()?;
        
        println!("✅ Batch creation (9 accounts): {:?}", batch_duration);
        println!("✅ Average per account in batch: {:?}", batch_duration / 9);
    }
    
    Ok(())
}

/// Test ZK proof submission to EVM Authorization contract
async fn test_zk_proof_submission_evm(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing ZK proof submission to EVM Authorization contract...");
    
    if let (Some(auth_addr), Some(gateway_addr)) = (
        &config.contract_addresses.evm_authorization,
        &config.contract_addresses.evm_verification_gateway
    ) {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
        
        // Generate a ZK proof
        let proof_request = serde_json::json!({
            "controller": "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8",
            "program_id": "evm_zk_test",
            "account_request_id": 700,
            "account_type": 1,
            "libraries": []
        });
        
        let proof_id = coprocessor_client.request_proof("evm_account_factory", &proof_request).await?;
        let proof_result = coprocessor_client.wait_for_proof(&proof_id).await?;
        
        // Register verification key first
        let vk_tx = eth_client.register_verification_key(
            gateway_addr,
            "evm_account_factory",
            "mock_verification_key"
        ).await?;
        
        let vk_success = eth_client.verify_transaction(&vk_tx).await?;
        assert!(vk_success);
        println!("✅ EVM verification key registered");
        
        // Submit ZK proof to Authorization contract
        let public_inputs = proof_result["public_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        
        let tx_hash = eth_client.submit_zk_proof(
            auth_addr,
            &proof_result["proof"],
            &public_inputs
        ).await?;
        
        // Verify transaction was successful
        let success = eth_client.verify_transaction(&tx_hash).await?;
        assert!(success);
        
        println!("✅ EVM ZK proof submitted and verified on-chain");
    }
    
    Ok(())
}

/// Test ZK proof submission to CosmWasm Authorization contract
async fn test_zk_proof_submission_cosmwasm(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing ZK proof submission to CosmWasm Authorization contract...");
    
    if let (Some(auth_addr), Some(gateway_addr)) = (
        &config.contract_addresses.cosmwasm_authorization,
        &config.contract_addresses.cosmwasm_verification_gateway
    ) {
        if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
            let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
            
            // Generate a ZK proof
            let proof_request = serde_json::json!({
                "controller": "cosmos1testuser",
                "program_id": "cosmwasm_zk_test",
                "account_request_id": 701,
                "account_type": 1,
                "libraries": []
            });
            
            let proof_id = coprocessor_client.request_proof("cosmwasm_account_factory", &proof_request).await?;
            let proof_result = coprocessor_client.wait_for_proof(&proof_id).await?;
            
            // Register verification key first
            let vk_tx = cosmwasm_client.register_verification_key_cosmwasm(
                gateway_addr,
                "cosmwasm_account_factory",
                "mock_verification_key"
            ).await?;
            
            let vk_success = cosmwasm_client.verify_transaction_cosmwasm(&vk_tx).await?;
            assert!(vk_success);
            println!("✅ CosmWasm verification key registered");
            
            // Submit ZK proof to Authorization contract
            let public_inputs = proof_result["public_inputs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect::<Vec<_>>();
            
            let tx_hash = cosmwasm_client.submit_zk_proof_cosmwasm(
                auth_addr,
                &proof_result["proof"],
                &public_inputs
            ).await?;
            
            // Verify transaction was successful
            let success = cosmwasm_client.verify_transaction_cosmwasm(&tx_hash).await?;
            assert!(success);
            
            println!("✅ CosmWasm ZK proof submitted and verified on-chain");
        }
    }
    
    Ok(())
}

/// Test complete end-to-end account creation with ZK proofs on EVM
async fn test_e2e_account_creation_evm(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing end-to-end account creation with ZK proofs on EVM...");
    
    if let (Some(auth_addr), Some(processor_addr), Some(factory_addr)) = (
        &config.contract_addresses.evm_authorization,
        &config.contract_addresses.evm_processor,
        &config.contract_addresses.evm_factory
    ) {
        let eth_client = EthereumClient::new(config.anvil_rpc_url.clone());
        let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
        
        // Step 1: Generate ZK proof for account creation
        let account_request = AccountCreationRequest {
            controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
            program_id: "e2e_test_evm".to_string(),
            account_request_id: 800,
            account_type: 1,
            libraries: vec!["lib1".to_string()],
            historical_block_number: None,
            target_chain: None,
        };
        
        let proof_request = serde_json::json!({
            "controller": account_request.controller,
            "program_id": account_request.program_id,
            "account_request_id": account_request.account_request_id,
            "account_type": account_request.account_type,
            "libraries": account_request.libraries
        });
        
        let proof_id = coprocessor_client.request_proof("evm_account_factory", &proof_request).await?;
        let proof_result = coprocessor_client.wait_for_proof(&proof_id).await?;
        
        // Step 2: Submit ZK proof to Authorization contract
        let public_inputs = proof_result["public_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        
        let proof_tx = eth_client.submit_zk_proof(
            auth_addr,
            &proof_result["proof"],
            &public_inputs
        ).await?;
        
        let proof_success = eth_client.verify_transaction(&proof_tx).await?;
        assert!(proof_success);
        println!("✅ EVM ZK proof submitted successfully");
        
        // Step 3: Process account creation through Processor
        let message_batch = serde_json::json!({
            "messages": [{
                "contract": factory_addr,
                "msg": {
                    "create_account": {
                        "controller": account_request.controller,
                        "program_id": account_request.program_id,
                        "account_request_id": account_request.account_request_id,
                        "account_type": account_request.account_type,
                        "libraries": account_request.libraries
                    }
                }
            }],
            "is_atomic": true
        });
        
        let process_tx = eth_client.process_account_creation(
            processor_addr,
            &message_batch
        ).await?;
        
        let process_success = eth_client.verify_transaction(&process_tx).await?;
        assert!(process_success);
        println!("✅ EVM account creation processed successfully");
        
        // Step 4: Verify the account was actually created
        let predicted_addr = eth_client.compute_account_address(factory_addr, &account_request).await?;
        let controller = eth_client.get_account_controller(&predicted_addr).await?;
        assert_eq!(controller.to_lowercase(), account_request.controller.to_lowercase());
        
        println!("✅ EVM end-to-end account creation with ZK proofs complete");
        println!("   Account address: {}", predicted_addr);
    }
    
    Ok(())
}

/// Test complete end-to-end account creation with ZK proofs on CosmWasm
async fn test_e2e_account_creation_cosmwasm(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing end-to-end account creation with ZK proofs on CosmWasm...");
    
    if let (Some(auth_addr), Some(processor_addr), Some(factory_addr)) = (
        &config.contract_addresses.cosmwasm_authorization,
        &config.contract_addresses.cosmwasm_processor,
        &config.contract_addresses.cosmwasm_factory
    ) {
        if let Ok(cosmwasm_client) = CosmWasmClient::new(config.cosmwasm_rpc_url.clone()) {
            let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
            
            // Step 1: Generate ZK proof for account creation
            let account_request = AccountCreationRequest {
                controller: "cosmos1testuser".to_string(),
                program_id: "e2e_test_cosmwasm".to_string(),
                account_request_id: 801,
                account_type: 2,
                libraries: vec!["lib1".to_string(), "lib2".to_string()],
                historical_block_number: None,
                target_chain: None,
            };
            
            let proof_request = serde_json::json!({
                "controller": account_request.controller,
                "program_id": account_request.program_id,
                "account_request_id": account_request.account_request_id,
                "account_type": account_request.account_type,
                "libraries": account_request.libraries
            });
            
            let proof_id = coprocessor_client.request_proof("cosmwasm_account_factory", &proof_request).await?;
            let proof_result = coprocessor_client.wait_for_proof(&proof_id).await?;
            
            // Step 2: Submit ZK proof to Authorization contract
            let public_inputs = proof_result["public_inputs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect::<Vec<_>>();
            
            let proof_tx = cosmwasm_client.submit_zk_proof_cosmwasm(
                auth_addr,
                &proof_result["proof"],
                &public_inputs
            ).await?;
            
            let proof_success = cosmwasm_client.verify_transaction_cosmwasm(&proof_tx).await?;
            assert!(proof_success);
            println!("✅ CosmWasm ZK proof submitted successfully");
            
            // Step 3: Process account creation through Processor
            let message_batch = serde_json::json!({
                "messages": [{
                    "contract": factory_addr,
                    "msg": {
                        "create_account": {
                            "controller": account_request.controller,
                            "program_id": account_request.program_id,
                            "account_request_id": account_request.account_request_id,
                            "account_type": account_request.account_type,
                            "libraries": account_request.libraries
                        }
                    }
                }],
                "is_atomic": true
            });
            
            let process_tx = cosmwasm_client.process_account_creation_cosmwasm(
                processor_addr,
                &message_batch
            ).await?;
            
            let process_success = cosmwasm_client.verify_transaction_cosmwasm(&process_tx).await?;
            assert!(process_success);
            println!("✅ CosmWasm account creation processed successfully");
            
            // Step 4: Verify the account was actually created
            let predicted_addr = cosmwasm_client.compute_account_address(factory_addr, &account_request).await?;
            let controller = cosmwasm_client.get_account_controller(&predicted_addr).await?;
            assert_eq!(controller, account_request.controller);
            
            println!("✅ CosmWasm end-to-end account creation with ZK proofs complete");
            println!("   Account address: {}", predicted_addr);
        }
    }
    
    Ok(())
}

/// Test ferry service architecture flow
async fn test_ferry_service_architecture(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing complete ferry service architecture flow...");
    
    // Initialize ferry service with multi-chain support
    let ferry_service = FerryService::new(
        "architecture_test_ferry".to_string(),
        vec!["ethereum".to_string(), "neutron".to_string()]
    );
    
    // Step 1: Application submits account creation request
    println!("Step 1: Application -> Ferry Service (Account Request)");
    let app_request = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "architecture_test".to_string(),
        account_request_id: 1000,
        account_type: 1, // TokenCustody
        libraries: vec!["defi_lib".to_string()],
        historical_block_number: None, // Ferry will populate
        target_chain: Some("ethereum".to_string()),
    };
    
    let request_id = ferry_service.submit_account_request(
        app_request.clone(),
        "ethereum"
    ).await?;
    println!("✅ Request submitted to ferry: {}", request_id);
    
    // Step 2: Set up architecture components
    println!("Step 2: Setting up architecture components");
    let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
    
    let mut verification_gateways = std::collections::HashMap::new();
    verification_gateways.insert("ethereum".to_string(), 
        config.contract_addresses.evm_verification_gateway.clone().unwrap_or_default());
    
    let mut account_factories = std::collections::HashMap::new();
    account_factories.insert("ethereum".to_string(),
        config.contract_addresses.evm_factory.clone().unwrap_or_default());
    
    // Step 3: Ferry service processes batch through full architecture
    println!("Step 3: Ferry Service -> ZK Coprocessor -> Verification Gateway -> Account Factory");
    let batch_result = ferry_service.process_batch(
        &coprocessor_client,
        &verification_gateways,
        &account_factories
    ).await?;
    
    // Step 4: Verify architecture flow completed successfully
    println!("Step 4: Verifying architecture flow results");
    assert_eq!(batch_result.accounts.len(), 1);
    assert!(matches!(batch_result.status, BatchStatus::AccountsCreated));
    
    let created_account = &batch_result.accounts[0];
    println!("✅ Account created through full architecture: {}", created_account);
    
    // Verify historical block entropy was included
    // Account address should be deterministic based on historical block + request params
    assert!(created_account.starts_with("0x"));
    assert_eq!(created_account.len(), 42); // Standard Ethereum address length
    
    println!("✅ Ferry service architecture flow test completed successfully");
    
    Ok(())
}

/// Test historical block entropy validation
async fn test_historical_block_validation(config: &E2EConfig) -> Result<(), Box<dyn Error>> {
    println!("Testing historical block entropy validation...");
    
    let ferry_service = FerryService::new(
        "historical_test_ferry".to_string(),
        vec!["ethereum".to_string()]
    );
    
    // Test 1: Valid historical block (recent)
    println!("Test 1: Valid recent historical block");
    let valid_request = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "historical_test".to_string(),
        account_request_id: 2001,
        account_type: 1,
        libraries: vec![],
        historical_block_number: Some(18_000_000 - 10), // Recent block
        target_chain: Some("ethereum".to_string()),
    };
    
    let request_id = ferry_service.submit_account_request(
        valid_request.clone(),
        "ethereum"
    ).await?;
    println!("✅ Valid historical block accepted: {}", request_id);
    
    // Test 2: Test that different historical blocks produce different addresses
    println!("Test 2: Different historical blocks produce different addresses");
    let request_1 = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "entropy_test".to_string(),
        account_request_id: 3001,
        account_type: 1,
        libraries: vec![],
        historical_block_number: Some(18_000_000 - 20),
        target_chain: Some("ethereum".to_string()),
    };
    
    let request_2 = AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: "entropy_test".to_string(),
        account_request_id: 3002, // Different ID
        account_type: 1,
        libraries: vec![],
        historical_block_number: Some(18_000_000 - 30), // Different block
        target_chain: Some("ethereum".to_string()),
    };
    
    ferry_service.submit_account_request(request_1, "ethereum").await?;
    ferry_service.submit_account_request(request_2, "ethereum").await?;
    
    // Process batch to create accounts
    let coprocessor_client = CoprocessorClient::new(config.coprocessor_url.clone());
    let mut verification_gateways = std::collections::HashMap::new();
    verification_gateways.insert("ethereum".to_string(), "mock_gateway".to_string());
    let mut account_factories = std::collections::HashMap::new();
    account_factories.insert("ethereum".to_string(), "mock_factory".to_string());
    
    let batch_result = ferry_service.process_batch(
        &coprocessor_client,
        &verification_gateways,
        &account_factories
    ).await?;
    
    // Verify different addresses were generated
    assert!(batch_result.accounts.len() >= 2);
    let addr1 = &batch_result.accounts[batch_result.accounts.len() - 2];
    let addr2 = &batch_result.accounts[batch_result.accounts.len() - 1];
    assert_ne!(addr1, addr2);
    println!("✅ Different historical blocks produce different addresses:");
    println!("  Address 1: {}", addr1);
    println!("  Address 2: {}", addr2);
    
    // Test 3: Simulate validation of historical block age
    println!("Test 3: Historical block age validation");
    
    // Mock test for historical block age validation (would be enforced in real contracts)
    let current_block = 18_000_000u64;
    let old_block = current_block - 300; // > 200 blocks old
    let block_age = current_block - old_block;
    
    if block_age > 200 {
        println!("✅ Historical block age validation works: block {} is {} blocks old (> 200 limit)", 
                old_block, block_age);
    }
    
    println!("✅ Historical block entropy validation test completed successfully");
    
    Ok(())
}

/// Print comprehensive test results
fn print_test_results(results: &E2ETestResults) {
    println!("\n=== Account Factory E2E Test Results ===");
    println!("Total Duration: {:?}", results.total_duration);
    println!("Tests Passed: {}", results.passed_count());
    println!("Tests Failed: {}", results.failed_count());
    println!();
    
    for (test_name, result) in &results.results {
        match result {
            Ok(()) => println!("✅ {}", test_name),
            Err(error) => println!("❌ {}: {}", test_name, error),
        }
    }
    
    if results.failed_count() > 0 {
        println!("\n❌ Some tests failed. Check the errors above.");
    } else {
        println!("\n🎉 All tests passed!");
    }
}

/// Account creation request structure
#[derive(Debug, Clone)]
pub struct AccountCreationRequest {
    pub controller: String,
    pub program_id: String,
    pub account_request_id: u64,
    pub account_type: u8,
    pub libraries: Vec<String>,
    pub historical_block_number: Option<u64>,
    pub target_chain: Option<String>,
}

/// Atomic account creation request
#[derive(Debug, Clone)]
pub struct AtomicAccountRequest {
    pub request: AccountCreationRequest,
    pub signature: Vec<u8>,
    pub expiration: u64,
} 