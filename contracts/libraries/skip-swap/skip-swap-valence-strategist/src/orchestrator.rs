use cosmwasm_std::{Addr, StdResult, Uint128};
use std::collections::HashMap;
use std::time::Duration;

use crate::chain::ChainClient;
use crate::skip::{SkipApiClient, create_execute_optimized_route_msg};

/// Configuration for the orchestrator
pub struct OrchestratorConfig {
    /// Library contract address
    pub library_address: Addr,
    
    /// Accounts to monitor for deposits
    pub monitored_accounts: HashMap<String, Addr>,
    
    /// Polling interval in seconds
    pub polling_interval: u64,
    
    /// Maximum number of retries for failed transactions
    pub max_retries: u8,
    
    /// Delay between retries in seconds
    pub retry_delay: u64,
    
    /// Skip API base URL
    pub skip_api_url: String,
}

/// Main orchestrator for the Strategist
pub struct Orchestrator<T: SkipApiClient> {
    /// Chain client for interacting with the blockchain
    pub chain_client: ChainClient,
    
    /// Skip API client for querying optimal routes
    pub skip_api_client: T,
    
    /// Configuration for the orchestrator
    pub config: OrchestratorConfig,
    
    /// Last polled balances for monitoring changes
    last_balances: HashMap<String, HashMap<String, Uint128>>,
}

impl<T: SkipApiClient> Orchestrator<T> {
    /// Create a new orchestrator
    pub fn new(chain_client: ChainClient, skip_api_client: T, config: OrchestratorConfig) -> Self {
        Self {
            chain_client,
            skip_api_client,
            config,
            last_balances: HashMap::new(),
        }
    }

    /// Start the polling loop
    #[cfg(feature = "runtime")]
    pub async fn start_polling(&mut self) -> StdResult<()> {
        loop {
            self.poll_for_deposits()?;
            
            // Use tokio's async sleep instead of blocking thread::sleep
            tokio::time::sleep(tokio::time::Duration::from_secs(self.config.polling_interval)).await;
        }
    }

    /// Synchronous version of start_polling for environments without async runtime
    #[cfg(not(feature = "runtime"))]
    pub fn start_polling(&mut self) -> StdResult<()> {
        loop {
            self.poll_for_deposits()?;
            
            // Use blocking sleep in non-async environments
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
    pub fn process_deposit(&self, _account: &Addr, token: &str, amount: Uint128) -> StdResult<String> {
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
        )?;
        
        // 4. Submit transaction with retry logic
        self.submit_with_retry(vec![msg])
    }

    /// Submit a transaction with retry logic
    pub fn submit_with_retry(&self, msgs: Vec<cosmwasm_std::CosmosMsg>) -> StdResult<String> {
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
    use crate::skip::MockSkipApiClient;

    #[test]
    fn test_orchestrator() {
        let chain_client = ChainClient::new(Addr::unchecked("strategist"));
        let skip_api_client = MockSkipApiClient{};
        
        let mut monitored_accounts = HashMap::new();
        monitored_accounts.insert("uusdc".to_string(), Addr::unchecked("account1"));
        
        let config = OrchestratorConfig {
            library_address: Addr::unchecked("library"),
            monitored_accounts,
            polling_interval: 10,
            max_retries: 3,
            retry_delay: 5,
            skip_api_url: "https://api.skip.money".to_string(),
        };
        
        let _orchestrator = Orchestrator::new(chain_client, skip_api_client, config);
        
        // In a real test, we would mock the chain_client and skip_api_client
        // and test the orchestrator logic more thoroughly
    }
} 