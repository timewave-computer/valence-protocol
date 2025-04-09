/*
 * Chain client for blockchain interactions.
 * Provides an interface for communicating with the blockchain network,
 * including transaction submission, balance queries, and contract interactions.
 * Supports both mock implementations for testing and actual blockchain clients.
 */

use cosmwasm_std::{Addr, Coin, StdResult, StdError, Uint128};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::str::FromStr;
use thiserror::Error;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::types::RouteParameters;

/// Errors that can occur when interacting with the blockchain
#[derive(Error, Debug)]
pub enum ChainError {
    #[error("HTTP error: {0}")]
    HttpError(String),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Query error: {0}")]
    QueryError(String),
    
    #[error("Transaction error: {0}")]
    TransactionError(String),
    
    #[error("Timeout waiting for transaction")]
    Timeout,
}

/// Balance query response from the chain
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BalanceResponse {
    pub amount: Coin,
}

/// Client for interacting with the blockchain
pub struct ChainClient {
    /// Address that will be used to sign transactions
    pub address: Addr,
    
    /// RPC endpoint for the chain
    #[cfg(feature = "runtime")]
    rpc_url: String,
    
    /// HTTP client for making requests
    #[cfg(feature = "runtime")]
    client: reqwest::Client,
    
    /// Chain ID (e.g., "neutron-1")
    #[cfg(feature = "runtime")]
    chain_id: String,
    
    /// Optional private key for signing transactions
    #[cfg(feature = "runtime")]
    private_key: Option<Vec<u8>>,
    
    /// Account sequence number for transactions
    #[cfg(feature = "runtime")]
    sequence: u64,
    
    /// Account number
    #[cfg(feature = "runtime")]
    account_number: u64,
}

impl ChainClient {
    /// Create a new chain client
    pub fn new(address: Addr) -> Self {
        #[cfg(feature = "runtime")]
        let client = reqwest::Client::new();
        
        Self { 
            address,
            #[cfg(feature = "runtime")]
            rpc_url: "http://localhost:26657".to_string(), // Default value
            #[cfg(feature = "runtime")]
            client,
            #[cfg(feature = "runtime")]
            chain_id: "neutron-1".to_string(), // Default value
            #[cfg(feature = "runtime")]
            private_key: None,
            #[cfg(feature = "runtime")]
            sequence: 0,
            #[cfg(feature = "runtime")]
            account_number: 0,
        }
    }
    
    /// Create a new chain client with custom configuration
    #[cfg(feature = "runtime")]
    pub fn new_with_config(address: Addr, rpc_url: String, chain_id: String) -> Self {
        let client = reqwest::Client::new();
        
        Self {
            address,
            rpc_url,
            client,
            chain_id,
            private_key: None,
            sequence: 0,
            account_number: 0,
        }
    }
    
    /// Set the private key for signing transactions
    #[cfg(feature = "runtime")]
    pub fn with_private_key(mut self, private_key: Vec<u8>) -> Self {
        self.private_key = Some(private_key);
        self
    }
    
    /// Set the sequence number for transactions
    #[cfg(feature = "runtime")]
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }
    
    /// Set the account number
    #[cfg(feature = "runtime")]
    pub fn with_account_number(mut self, account_number: u64) -> Self {
        self.account_number = account_number;
        self
    }
    
    /// Initialize the chain client by fetching account details
    #[cfg(feature = "runtime")]
    pub async fn initialize(&mut self) -> StdResult<()> {
        // Fetch the account details to get sequence and account number
        let account_url = format!(
            "{}/cosmos/auth/v1beta1/accounts/{}",
            self.rpc_url.replace("26657", "1317"),
            self.address
        );
        
        let response = self.client.get(&account_url)
            .send()
            .await
            .map_err(|e| StdError::generic_err(format!("Failed to fetch account details: {}", e)))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(StdError::generic_err(format!("Error fetching account details: {}", error_text)));
        }
        
        let account_data: serde_json::Value = response.json().await
            .map_err(|e| StdError::generic_err(format!("Failed to parse account response: {}", e)))?;
            
        // Extract account number
        self.account_number = account_data["account"]["account_number"]
            .as_str()
            .ok_or_else(|| StdError::generic_err("Invalid account response: missing account_number"))?
            .parse::<u64>()
            .map_err(|e| StdError::generic_err(format!("Failed to parse account number: {}", e)))?;
            
        // Extract sequence number
        self.sequence = account_data["account"]["sequence"]
            .as_str()
            .ok_or_else(|| StdError::generic_err("Invalid account response: missing sequence"))?
            .parse::<u64>()
            .map_err(|e| StdError::generic_err(format!("Failed to parse sequence number: {}", e)))?;
            
        Ok(())
    }

    /// Query the balance of a token for an account
    pub fn query_balance(&self, account: &Addr, denom: &str) -> StdResult<Coin> {
        #[cfg(feature = "runtime")]
        {
            // Real implementation - needs to be executed in an async context
            // The caller should handle this by using tokio::runtime or #[tokio::main]
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                StdError::generic_err(format!("Failed to create tokio runtime: {}", e))
            })?;
            
            rt.block_on(async {
                self.query_balance_async(account, denom).await
            })
        }
        
        #[cfg(not(feature = "runtime"))]
        {
            // For testing or environments without async runtime
            Ok(Coin {
                denom: denom.to_string(),
                amount: Uint128::from(1000000u128),
            })
        }
    }
    
    /// Async version of query_balance
    #[cfg(feature = "runtime")]
    pub async fn query_balance_async(&self, account: &Addr, denom: &str) -> StdResult<Coin> {
        // Build the query URL for the bank module
        let query_url = format!(
            "{}/cosmos/bank/v1beta1/balances/{}/by_denom?denom={}",
            self.rpc_url.replace("26657", "1317"), // Use the REST endpoint instead of RPC
            account,
            denom
        );
        
        // Make the HTTP request
        let response = self.client.get(&query_url)
            .send()
            .await
            .map_err(|e| {
                StdError::generic_err(format!("HTTP error querying balance: {}", e))
            })?;
        
        // Check for errors
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(StdError::generic_err(format!("Error querying balance: {}", error_text)));
        }
        
        // Parse the response
        let balance_response: serde_json::Value = response.json().await.map_err(|e| {
            StdError::generic_err(format!("Error parsing balance response: {}", e))
        })?;
        
        // Extract the balance from the response
        let amount_str = balance_response["balance"]["amount"]
            .as_str()
            .ok_or_else(|| StdError::generic_err("Invalid balance response format"))?;
        
        // Parse the amount using u128::from_str
        let amount_u128 = u128::from_str(amount_str).map_err(|_| {
            StdError::generic_err(format!("Invalid amount in balance response: {}", amount_str))
        })?;
        
        Ok(Coin {
            denom: denom.to_string(),
            amount: Uint128::from(amount_u128),
        })
    }

    /// Submit a transaction to the chain
    pub fn submit_transaction(
        &mut self,
        msgs: Vec<cosmwasm_std::CosmosMsg>,
    ) -> StdResult<String> {
        #[cfg(feature = "runtime")]
        {
            // Real implementation using tokio runtime
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                StdError::generic_err(format!("Failed to create tokio runtime: {}", e))
            })?;
            
            rt.block_on(async {
                self.submit_transaction_async(msgs).await
            })
        }
        
        #[cfg(not(feature = "runtime"))]
        {
            // Generate a pseudo-random transaction hash for testing
            use sha2::{Sha256, Digest};
            
            // Create a deterministic but unique hash based on the messages
            let mut hasher = Sha256::new();
            for msg in &msgs {
                // Use the debug representation of the message to create a unique input
                hasher.update(format!("{:?}", msg).as_bytes());
            }
            // Add a timestamp to ensure uniqueness
            hasher.update(format!("{:?}", std::time::SystemTime::now()).as_bytes());
            
            let result = hasher.finalize();
            Ok(format!("{:x}", result))
        }
    }
    
    /// Async version of submit_transaction
    #[cfg(feature = "runtime")]
    pub async fn submit_transaction_async(
        &mut self,
        msgs: Vec<cosmwasm_std::CosmosMsg>,
    ) -> StdResult<String> {
        // Transform CosmosMsg to chain-specific messages
        let tx_body = self.prepare_transaction_body(msgs)?;
        
        // Sign the transaction
        let signed_tx = self.sign_transaction(tx_body)?;
        
        // Broadcast the transaction
        let tx_response = self.broadcast_transaction(signed_tx).await?;
        
        // Parse the response and extract the transaction hash
        let tx_hash = tx_response["tx_response"]["txhash"]
            .as_str()
            .ok_or_else(|| StdError::generic_err("Failed to get transaction hash from response"))?
            .to_string();
        
        // Increment sequence for next transaction
        self.sequence += 1;
        
        Ok(tx_hash)
    }
    
    /// Prepare a transaction body from CosmosMsg objects
    /// 
    /// This method converts CosmosMsg objects to the specific format required by the chain's transaction API.
    /// It supports Wasm messages for executing smart contracts with proper encoding of message parameters.
    #[cfg(feature = "runtime")]
    fn prepare_transaction_body(&self, msgs: Vec<cosmwasm_std::CosmosMsg>) -> StdResult<serde_json::Value> {
        let mut json_msgs = Vec::with_capacity(msgs.len());
        
        for msg in msgs {
            match msg {
                cosmwasm_std::CosmosMsg::Wasm(wasm_msg) => {
                    match wasm_msg {
                        cosmwasm_std::WasmMsg::Execute { contract_addr, msg, funds } => {
                            let json_msg = serde_json::json!({
                                "@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
                                "sender": self.address.to_string(),
                                "contract": contract_addr,
                                "msg": serde_json::from_slice::<serde_json::Value>(&msg).map_err(|e| {
                                    StdError::generic_err(format!("Failed to parse message: {}", e))
                                })?,
                                "funds": funds.iter().map(|coin| {
                                    serde_json::json!({
                                        "denom": coin.denom,
                                        "amount": coin.amount.to_string()
                                    })
                                }).collect::<Vec<_>>()
                            });
                            json_msgs.push(json_msg);
                        },
                        _ => return Err(StdError::generic_err("Unsupported WasmMsg type"))
                    }
                },
                _ => return Err(StdError::generic_err("Unsupported CosmosMsg type"))
            }
        }
        
        Ok(serde_json::json!({
            "messages": json_msgs,
            "memo": "Skip Swap Valence Strategist transaction",
            "timeout_height": 0
        }))
    }
    
    /// Sign a transaction using secp256k1
    #[cfg(feature = "runtime")]
    fn sign_transaction(&self, tx_body: serde_json::Value) -> StdResult<serde_json::Value> {
        // Check if we have a private key
        let private_key = match &self.private_key {
            Some(key) => key,
            None => return Err(StdError::generic_err("No private key available for signing")),
        };
        
        // Create the auth info object
        let auth_info = serde_json::json!({
            "signer_infos": [{
                "public_key": {
                    "@type": "/cosmos.crypto.secp256k1.PubKey",
                    "key": self.derive_pubkey_from_privkey(private_key)?,
                },
                "mode_info": {
                    "single": {
                        "mode": "SIGN_MODE_DIRECT"
                    }
                },
                "sequence": self.sequence.to_string()
            }],
            "fee": {
                "amount": [{"denom": "untrn", "amount": "1000"}],
                "gas_limit": "200000",
                "payer": "",
                "granter": ""
            }
        });
        
        // Create the sign doc
        let sign_doc = serde_json::json!({
            "body_bytes": self.serialize_canonical(&tx_body)?,
            "auth_info_bytes": self.serialize_canonical(&auth_info)?,
            "chain_id": self.chain_id,
            "account_number": self.account_number.to_string()
        });
        
        // Serialize the sign doc
        let sign_doc_bytes = self.serialize_canonical(&sign_doc)?;
        
        // Sign the transaction using secp256k1
        let signature = self.sign_bytes(&sign_doc_bytes, private_key)?;
        
        // Create the complete transaction
        Ok(serde_json::json!({
            "body": tx_body,
            "auth_info": auth_info,
            "signatures": [BASE64.encode(signature)]
        }))
    }
    
    /// Derive public key from private key
    #[cfg(feature = "runtime")]
    fn derive_pubkey_from_privkey(&self, private_key: &[u8]) -> StdResult<String> {
        use secp256k1::{Secp256k1, SecretKey, PublicKey};
        
        // Initialize secp256k1 context
        let secp = Secp256k1::new();
        
        // Create the private key
        let secret_key = SecretKey::from_slice(private_key)
            .map_err(|e| StdError::generic_err(format!("Invalid private key: {}", e)))?;
            
        // Derive the public key
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        
        // Convert to compressed form and encode as Base64
        Ok(BASE64.encode(public_key.serialize()))
    }
    
    /// Sign bytes using secp256k1
    #[cfg(feature = "runtime")]
    fn sign_bytes(&self, bytes: &[u8], private_key: &[u8]) -> StdResult<Vec<u8>> {
        use secp256k1::{Secp256k1, SecretKey, Message};
        use sha2::{Sha256, Digest};
        
        // Hash the bytes with SHA-256
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hashed = hasher.finalize();
        
        // Initialize secp256k1 context
        let secp = Secp256k1::new();
        
        // Create the private key
        let secret_key = SecretKey::from_slice(private_key)
            .map_err(|e| StdError::generic_err(format!("Invalid private key: {}", e)))?;
            
        // Create a message from the hash
        let message = Message::from_slice(hashed.as_slice())
            .map_err(|e| StdError::generic_err(format!("Failed to create message: {}", e)))?;
            
        // Sign the message
        let signature = secp.sign_ecdsa(&message, &secret_key);
        
        // Return the signature in DER format
        Ok(signature.serialize_der().to_vec())
    }
    
    /// Serialize a JSON value in canonical form
    #[cfg(feature = "runtime")]
    fn serialize_canonical(&self, value: &serde_json::Value) -> StdResult<Vec<u8>> {
        serde_json::to_vec(value)
            .map_err(|e| StdError::generic_err(format!("Failed to serialize to canonical JSON: {}", e)))
    }

    /// Broadcast a transaction to the chain
    #[cfg(feature = "runtime")]
    async fn broadcast_transaction(&self, signed_tx: serde_json::Value) -> StdResult<serde_json::Value> {
        // Construct the broadcast endpoint URL
        let broadcast_url = format!("{}/cosmos/tx/v1beta1/txs", self.rpc_url.replace("26657", "1317"));
        
        // Convert the signed transaction to a base64 string
        let tx_bytes = BASE64.encode(serde_json::to_string(&signed_tx).map_err(|e| {
            StdError::generic_err(format!("Failed to serialize transaction: {}", e))
        })?);
        
        // Create the request body
        let request_body = serde_json::json!({
            "tx_bytes": tx_bytes,
            "mode": "BROADCAST_MODE_SYNC"
        });
        
        // Make the HTTP request
        let response = self.client.post(&broadcast_url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                StdError::generic_err(format!("HTTP error broadcasting transaction: {}", e))
            })?;
        
        // Check for HTTP errors
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(StdError::generic_err(format!("Error broadcasting transaction: {}", error_text)));
        }
        
        // Parse the response
        let tx_response: serde_json::Value = response.json().await.map_err(|e| {
            StdError::generic_err(format!("Error parsing broadcast response: {}", e))
        })?;
        
        // Check for transaction errors
        if let Some(code) = tx_response["tx_response"]["code"].as_i64() {
            if code != 0 {
                let raw_log = tx_response["tx_response"]["raw_log"]
                    .as_str()
                    .unwrap_or("Unknown error");
                return Err(StdError::generic_err(format!("Transaction failed: {} (code: {})", raw_log, code)));
            }
        }
        
        // Return the transaction response
        Ok(tx_response)
    }

    /// Wait for a transaction to be confirmed
    pub fn wait_for_transaction(&mut self, tx_hash: &str) -> StdResult<bool> {
        #[cfg(feature = "runtime")]
        {
            // Real implementation using tokio runtime
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                StdError::generic_err(format!("Failed to create tokio runtime: {}", e))
            })?;
            
            rt.block_on(async {
                self.wait_for_transaction_async(tx_hash).await
            })
        }
        
        #[cfg(not(feature = "runtime"))]
        {
            // Mock implementation for testing
            Ok(true)
        }
    }
    
    /// Async version of wait_for_transaction
    #[cfg(feature = "runtime")]
    pub async fn wait_for_transaction_async(&mut self, tx_hash: &str) -> StdResult<bool> {
        // Construct the query URL for the transaction
        let query_url = format!(
            "{}/cosmos/tx/v1beta1/txs/{}",
            self.rpc_url.replace("26657", "1317"), 
            tx_hash
        );
        
        // Define maximum retries and delay between attempts
        const MAX_RETRIES: u8 = 30;  // Try for about 2 minutes
        const RETRY_DELAY: Duration = Duration::from_secs(4);
        
        // Poll for the transaction status
        for _ in 0..MAX_RETRIES {
            // Make the HTTP request
            let response = match self.client.get(&query_url).send().await {
                Ok(resp) => resp,
                Err(_) => {
                    // If we get an error, wait and retry
                    tokio::time::sleep(RETRY_DELAY).await;
                    continue;
                }
            };
            
            // If status is 404, the transaction is not yet in the chain
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }
            
            // For other error codes, check the response
            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                // If the error indicates that the transaction is not found, it's not an error
                if error_text.contains("not found") {
                    tokio::time::sleep(RETRY_DELAY).await;
                    continue;
                }
                // Otherwise, it's a real error
                return Err(StdError::generic_err(format!("Error querying transaction: {}", error_text)));
            }
            
            // Parse the response
            let tx_response: serde_json::Value = match response.json().await {
                Ok(resp) => resp,
                Err(e) => {
                    return Err(StdError::generic_err(format!("Error parsing transaction response: {}", e)));
                }
            };
            
            // Check if the transaction is confirmed
            let tx_code = tx_response["tx_response"]["code"].as_i64().unwrap_or(0);
            
            // If code is 0, the transaction is successful
            if tx_code == 0 {
                return Ok(true);
            }
            
            // If code is not 0, the transaction failed
            let raw_log = tx_response["tx_response"]["raw_log"]
                .as_str()
                .unwrap_or("Unknown error");
                
            return Err(StdError::generic_err(format!("Transaction failed: {}", raw_log)));
        }
        
        // If we've reached here, we've exceeded MAX_RETRIES
        Err(StdError::generic_err("Timeout waiting for transaction confirmation"))
    }

    /// Query the route parameters for a swap from a contract
    pub fn query_route_parameters(
        &self,
        contract_addr: &Addr,
        input_denom: &str,
        input_amount: Uint128,
    ) -> StdResult<RouteParameters> {
        #[cfg(feature = "runtime")]
        {
            // Real implementation using tokio runtime
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                StdError::generic_err(format!("Failed to create tokio runtime: {}", e))
            })?;
            
            rt.block_on(async {
                self.query_route_parameters_async(contract_addr, input_denom, input_amount).await
            })
        }
        
        #[cfg(not(feature = "runtime"))]
        {
            // Mock implementation for testing
            Ok(RouteParameters {
                allowed_asset_pairs: vec![],
                allowed_venues: vec!["astroport".to_string()],
                max_slippage: cosmwasm_std::Decimal::percent(1),
                token_destinations: Default::default(),
                intermediate_accounts: Default::default(),
            })
        }
    }
    
    /// Async version of query_route_parameters
    #[cfg(feature = "runtime")]
    pub async fn query_route_parameters_async(
        &self,
        contract_addr: &Addr,
        input_denom: &str,
        input_amount: Uint128,
    ) -> StdResult<RouteParameters> {
        // Construct the query
        let query_msg = serde_json::json!({
            "route_parameters": {
                "input_denom": input_denom,
                "input_amount": input_amount.to_string()
            }
        });
        
        // Encode the query for the REST API
        let encoded_query = BASE64.encode(serde_json::to_string(&query_msg).map_err(|e| {
            StdError::generic_err(format!("Failed to serialize query message: {}", e))
        })?);
        
        // Build the query URL
        let query_url = format!(
            "{}/cosmwasm/wasm/v1/contract/{}/smart/{}",
            self.rpc_url.replace("26657", "1317"),
            contract_addr,
            encoded_query
        );
        
        // Make the HTTP request
        let response = self.client.get(&query_url)
            .send()
            .await
            .map_err(|e| {
                StdError::generic_err(format!("HTTP error querying contract: {}", e))
            })?;
        
        // Check for errors
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(StdError::generic_err(format!("Error querying contract: {}", error_text)));
        }
        
        // Parse the response
        let query_response: serde_json::Value = response.json().await.map_err(|e| {
            StdError::generic_err(format!("Error parsing contract query response: {}", e))
        })?;
        
        // Extract the result data and decode it
        let result_data = query_response["data"]
            .as_str()
            .ok_or_else(|| StdError::generic_err("Invalid response format from contract query"))?;
             
        let decoded_result = BASE64.decode(result_data).map_err(|e| {
            StdError::generic_err(format!("Failed to decode contract query result: {}", e))
        })?;
        
        // Parse the decoded result as RouteParameters
        let route_params: RouteParameters = serde_json::from_slice(&decoded_result).map_err(|e| {
            StdError::generic_err(format!("Failed to parse route parameters: {}", e))
        })?;
        
        Ok(route_params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_balance() {
        let _client = ChainClient::new(Addr::unchecked("address"));
        
        #[cfg(not(feature = "runtime"))]
        {
            // This will use the mock implementation which doesn't make HTTP requests
            let balance = _client.query_balance(&Addr::unchecked("address"), "uusdc").unwrap();
            assert_eq!(balance.amount, Uint128::from(1000000u128));
        }
        
        #[cfg(feature = "runtime")]
        {
            // Skip this test when running with the runtime feature
            // as it would try to make an actual HTTP request
            println!("Skipping test_query_balance in runtime mode");
        }
    }
} 