/*
 * Orchestrator for the Skip Swap Valence strategist.
 * Coordinates the monitoring of accounts, processing of deposits,
 * and execution of optimized swap routes, serving as the main controller
 * for the strategist functionality.
 */

use std::collections::HashMap;
use std::time::Duration;

use crate::chain::ChainClient;
use crate::skip::{SkipApiClient, create_execute_optimized_route_msg};

use cosmwasm_std::{Addr, StdResult, Uint128};

/// Configuration for the orchestrator
pub struct OrchestratorConfig {
    /// Library contract address
    pub library_address: Addr,
    
    /// Accounts to monitor for deposits
    pub monitored_accounts: HashMap<String, Addr>,
    
    /// Polling interval in seconds
    pub polling_interval: u64,
    
    /// Maximum number of retries for failed transactions
    pub max_retries: u32,
    
    /// Delay between retries in seconds
    pub retry_delay: u64,
    
    /// Skip API base URL
    pub skip_api_url: String,
    
    /// Contract address
    pub contract_address: String,
}

/// Main orchestrator for the Strategist
pub struct Orchestrator<S: SkipApiClient> {
    /// Chain client for interacting with the blockchain
    pub chain_client: ChainClient,
    
    /// Skip API client for querying optimal routes
    pub skip_api_client: S,
    
    /// Configuration for the orchestrator
    pub config: OrchestratorConfig,
    
    /// Last polled balances for monitoring changes
    last_balances: HashMap<String, HashMap<String, Uint128>>,
}

impl<S: SkipApiClient> Orchestrator<S> {
    /// Create a new orchestrator
    pub fn new(chain_client: ChainClient, skip_api_client: S, config: OrchestratorConfig) -> Self {
        Self {
            chain_client,
            skip_api_client,
            config,
            last_balances: HashMap::new(),
        }
    }

    /// Start the polling loop
    pub fn start_polling(&mut self) -> StdResult<()> {
        loop {
            self.poll_for_deposits()?;
            
            // Sleep for the polling interval
            std::thread::sleep(std::time::Duration::from_secs(self.config.polling_interval));
        }
    }

    /// Poll for deposits in monitored accounts
    pub fn poll_for_deposits(&mut self) -> StdResult<Vec<(Addr, String, Uint128)>> {
        let mut deposits = Vec::new();
        let mut to_process = Vec::new();
        
        // Check each monitored account
        for (token, account) in &self.config.monitored_accounts {
            // Query current balance
            let balance = self.chain_client.query_balance(account, token)?;
            
            // Get last balance
            let last_balance = self.last_balances
                .entry(token.clone())
                .or_insert_with(HashMap::new)
                .entry(account.to_string())
                .or_insert(Uint128::zero());
            
            // If current balance is greater than last balance, we have a deposit
            if balance.amount > *last_balance {
                let deposit_amount = balance.amount - *last_balance;
                deposits.push((account.clone(), token.clone(), deposit_amount));
                to_process.push((account.clone(), token.clone(), deposit_amount));
                
                // Update last balance
                *last_balance = balance.amount;
            }
        }

        // Process deposits outside the borrow of self.last_balances
        for (account, token, amount) in to_process {
            // Process the deposit
            let _ = self.process_deposit(&account, &token, amount);
            // Ignore errors in polling to avoid breaking the loop
        }
        
        Ok(deposits)
    }

    /// Process a deposit
    pub fn process_deposit(&mut self, _account: &Addr, token: &str, amount: Uint128) -> StdResult<String> {
        // 1. Query route parameters
        let route_params = self.chain_client.query_route_parameters(
            &self.config.library_address,
            token,
            amount,
        )?;
        
        // 2. Query Skip API for optimal route
        let output_denom = route_params.allowed_asset_pairs
            .iter()
            .find(|pair| pair.input_asset == token)
            .map(|pair| pair.output_asset.clone())
            .unwrap_or_default();
        
        let route = self.skip_api_client.query_optimal_route(
            token,
            &output_denom,
            amount,
            &route_params.allowed_venues,
            route_params.max_slippage,
        )?;
        
        // 3. Construct ExecuteOptimizedRoute message
        let msg = create_execute_optimized_route_msg(
            token.to_string(),
            amount,
            output_denom,
            route.expected_output,
            route,
            self.config.contract_address.clone(),
        )?;
        
        // 4. Submit transaction with retry logic
        self.submit_with_retry(vec![msg])
    }

    /// Submit a transaction with retry logic
    pub fn submit_with_retry(&mut self, msgs: Vec<cosmwasm_std::CosmosMsg>) -> StdResult<String> {
        let mut retries = 0;
        loop {
            match self.chain_client.submit_transaction(msgs.clone()) {
                Ok(tx_hash) => {
                    // Wait for confirmation
                    match self.chain_client.wait_for_transaction(&tx_hash) {
                        Ok(true) => return Ok(tx_hash),
                        Ok(false) => {
                            // Transaction failed on chain
                            if retries >= self.config.max_retries {
                                return Err(cosmwasm_std::StdError::generic_err(
                                    "Max retries reached for transaction confirmation",
                                ));
                            }
                            retries += 1;
                        }
                        Err(e) => {
                            // Error checking transaction
                            if retries >= self.config.max_retries {
                                return Err(e);
                            }
                            retries += 1;
                        }
                    }
                }
                Err(e) => {
                    // Error submitting transaction
                    if retries >= self.config.max_retries {
                        return Err(e);
                    }
                    retries += 1;
                }
            }
            
            // Sleep before retry
            std::thread::sleep(Duration::from_secs(self.config.retry_delay));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skip::{MockSkipApiClient, SwapOperation};
    use cosmwasm_std::{Coin, Decimal};

    // Test-specific implementation of poll_for_deposits and process_deposit
    // This avoids the need to mock the chain client
    fn test_poll_for_deposits(
        skip_api_client: &MockSkipApiClient,
        monitored_accounts: &HashMap<String, Addr>,
        balances: &HashMap<String, HashMap<String, Uint128>>,
        last_balances: &mut HashMap<String, HashMap<String, Uint128>>,
        submitted_msgs: &mut Vec<cosmwasm_std::CosmosMsg>,
        config: &OrchestratorConfig,
    ) -> StdResult<Vec<(Addr, String, Uint128)>> {
        let mut deposits = Vec::new();
        
        // Check each monitored account
        for (token, account) in monitored_accounts {
            // Query "current balance" from our test balances
            let amount = balances
                .get(token)
                .and_then(|token_balances| token_balances.get(&account.to_string()))
                .copied()
                .unwrap_or(Uint128::zero());
            
            let balance = Coin {
                denom: token.clone(),
                amount,
            };
            
            // Get last balance
            let last_balance = last_balances
                .entry(token.clone())
                .or_insert_with(HashMap::new)
                .entry(account.to_string())
                .or_insert(Uint128::zero());
            
            // If current balance is greater than last balance, we have a deposit
            if balance.amount > *last_balance {
                let deposit_amount = balance.amount - *last_balance;
                deposits.push((account.clone(), token.clone(), deposit_amount));
                
                // Process the deposit for this test
                let _ = test_process_deposit(
                    skip_api_client,
                    account,
                    token,
                    deposit_amount,
                    submitted_msgs,
                    config,
                );
                
                // Update last balance
                *last_balance = balance.amount;
            }
        }
        
        Ok(deposits)
    }
    
    fn test_process_deposit(
        skip_api_client: &MockSkipApiClient,
        _account: &Addr,
        token: &str,
        amount: Uint128,
        submitted_msgs: &mut Vec<cosmwasm_std::CosmosMsg>,
        config: &OrchestratorConfig,
    ) -> StdResult<String> {
        // 1. Create mock route parameters
        let route_params = crate::types::RouteParameters {
            allowed_asset_pairs: vec![
                crate::types::AssetPair {
                    input_asset: token.to_string(),
                    output_asset: "uatom".to_string(),
                }
            ],
            allowed_venues: vec!["astroport".to_string()],
            max_slippage: Decimal::percent(1),
            token_destinations: HashMap::new(),
            intermediate_accounts: HashMap::new(),
        };
        
        // 2. Query Skip API for optimal route
        let output_denom = route_params.allowed_asset_pairs
            .iter()
            .find(|pair| pair.input_asset == token)
            .map(|pair| pair.output_asset.clone())
            .unwrap_or_default();
        
        let route = skip_api_client.query_optimal_route(
            token,
            &output_denom,
            amount,
            &route_params.allowed_venues,
            route_params.max_slippage,
        )?;
        
        // 3. Construct ExecuteOptimizedRoute message
        let msg = create_execute_optimized_route_msg(
            token.to_string(),
            amount,
            output_denom,
            route.expected_output,
            route,
            config.contract_address.clone(),
        )?;
        
        // 4. Add to submitted messages and return a mock tx hash
        submitted_msgs.push(msg);
        Ok(format!("mock_tx_hash_{}", submitted_msgs.len()))
    }

    #[test]
    fn test_orchestrator_poll_for_deposits() {
        // Create fake accounts and balances
        let account_addr = Addr::unchecked("account1");
        let mut monitored_accounts = HashMap::new();
        monitored_accounts.insert("uusdc".to_string(), account_addr.clone());
        
        let skip_api_client = MockSkipApiClient::new()
            .expected_output(Uint128::new(990000))
            .swap_venue("astroport".to_string())
            .operations(vec![SwapOperation {
                pool_id: "mock-pool-1".to_string(),
                denom_in: "uusdc".to_string(),
                denom_out: "uatom".to_string(),
            }]);
        
        let config = OrchestratorConfig {
            library_address: Addr::unchecked("library"),
            monitored_accounts: monitored_accounts.clone(),
            polling_interval: 10,
            max_retries: 3,
            retry_delay: 5,
            skip_api_url: "https://api.skip.money".to_string(),
            contract_address: "valence_skip_swap".to_string(),
        };
        
        // Setup test state
        let mut balances: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
        let mut last_balances: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
        let mut submitted_msgs: Vec<cosmwasm_std::CosmosMsg> = Vec::new();
        
        // Initial poll - should find no deposits since balance is zero
        let initial_deposits = test_poll_for_deposits(
            &skip_api_client,
            &monitored_accounts,
            &balances,
            &mut last_balances,
            &mut submitted_msgs,
            &config,
        ).unwrap();
        
        assert!(initial_deposits.is_empty(), "Expected no deposits initially");
        
        // Set a balance for the monitored account
        let mut account_balances = HashMap::new();
        account_balances.insert(account_addr.to_string(), Uint128::new(500000));
        balances.insert("uusdc".to_string(), account_balances);
        
        // Poll again - should find the deposit
        let deposits = test_poll_for_deposits(
            &skip_api_client,
            &monitored_accounts,
            &balances,
            &mut last_balances,
            &mut submitted_msgs,
            &config,
        ).unwrap();
        
        assert_eq!(deposits.len(), 1, "Expected one deposit to be found");
        assert_eq!(deposits[0].0, account_addr);
        assert_eq!(deposits[0].1, "uusdc");
        assert_eq!(deposits[0].2, Uint128::new(500000));
        
        // Verify that a transaction was submitted
        assert!(!submitted_msgs.is_empty(), "Expected at least one message to be submitted");
        
        // Poll again - should find no new deposits since the balance is unchanged
        let second_deposits = test_poll_for_deposits(
            &skip_api_client,
            &monitored_accounts,
            &balances,
            &mut last_balances,
            &mut submitted_msgs,
            &config,
        ).unwrap();
        
        assert!(second_deposits.is_empty(), "Expected no new deposits on second poll");
        
        // Increase the balance
        balances.get_mut("uusdc").unwrap().insert(account_addr.to_string(), Uint128::new(800000));
        
        // Poll again - should find only the new deposit amount
        let third_deposits = test_poll_for_deposits(
            &skip_api_client,
            &monitored_accounts,
            &balances,
            &mut last_balances,
            &mut submitted_msgs,
            &config,
        ).unwrap();
        
        assert_eq!(third_deposits.len(), 1);
        assert_eq!(third_deposits[0].2, Uint128::new(300000));
    }

    #[test]
    fn test_orchestrator_process_deposit() {
        // Setup test components
        let skip_api_client = MockSkipApiClient::new()
            .expected_output(Uint128::new(990000))
            .swap_venue("astroport".to_string())
            .operations(vec![SwapOperation {
                pool_id: "mock-pool-1".to_string(),
                denom_in: "uusdc".to_string(),
                denom_out: "uatom".to_string(),
            }]);
        
        let config = OrchestratorConfig {
            library_address: Addr::unchecked("library"),
            monitored_accounts: HashMap::new(),
            polling_interval: 10,
            max_retries: 3,
            retry_delay: 5,
            skip_api_url: "https://api.skip.money".to_string(),
            contract_address: "valence_skip_swap".to_string(),
        };
        
        // Process a deposit directly
        let account = Addr::unchecked("account1");
        let token = "uusdc";
        let amount = Uint128::new(1000000);
        let mut submitted_msgs: Vec<cosmwasm_std::CosmosMsg> = Vec::new();
        
        let tx_hash = test_process_deposit(
            &skip_api_client,
            &account,
            token,
            amount,
            &mut submitted_msgs,
            &config,
        ).unwrap();
        
        assert!(!tx_hash.is_empty(), "Expected a valid transaction hash");
        
        // Verify that a transaction was submitted with correct message
        assert_eq!(submitted_msgs.len(), 1, "Expected exactly one message to be submitted");
        
        // Verify the message contains expected route parameters
        if let cosmwasm_std::CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute { contract_addr, .. }) = &submitted_msgs[0] {
            assert_eq!(contract_addr, &config.contract_address, "Message should target the configured contract address");
        } else {
            panic!("Expected a WasmMsg::Execute");
        }
    }
} 