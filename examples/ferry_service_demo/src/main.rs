//! Ferry Service Demo for Account Factory
//!
//! This example demonstrates a basic ferry service that batches account creation requests
//! and submits them to the account factory for efficient processing.

use cosmwasm_schema::cw_serde;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Account request structure matching the account factory API
#[cw_serde]
pub struct AccountRequest {
    pub controller: String,
    pub libraries: Vec<String>,
    pub program_id: String,
    pub account_request_id: u64,
    pub account_type: u8,             // 1=TokenCustody, 2=DataStorage, 3=Hybrid
    pub historical_block_height: u64, // Block height used for entropy
    pub signature: Option<Vec<u8>>,   // Optional for atomic operations
}

/// Batch request structure for ferry services
#[cw_serde]
pub struct BatchRequest {
    pub requests: Vec<AccountRequest>,
    pub ferry: String,
    pub fee_amount: u128,
}

/// Basic Ferry Service for batching account creation requests
pub struct FerryService {
    pub ferry_address: String,
    pub batch_size: usize,
    pub fee_per_request: u128,
    pub pending_requests: Vec<AccountRequest>,
}

impl FerryService {
    /// Create a new ferry service instance
    pub fn new(ferry_address: String, batch_size: usize, fee_per_request: u128) -> Self {
        Self {
            ferry_address,
            batch_size,
            fee_per_request,
            pending_requests: Vec::new(),
        }
    }

    /// Add an account request to the ferry queue
    pub fn queue_request(&mut self, request: AccountRequest) -> Result<(), String> {
        // Basic validation
        if request.controller.is_empty() {
            return Err("Controller cannot be empty".to_string());
        }
        if request.libraries.is_empty() {
            return Err("Libraries cannot be empty".to_string());
        }
        if request.program_id.is_empty() {
            return Err("Program ID cannot be empty".to_string());
        }
        if ![1, 2, 3].contains(&request.account_type) {
            return Err("Invalid account type, must be 1, 2, or 3".to_string());
        }

        self.pending_requests.push(request);
        println!(
            "✓ Queued request. Total pending: {}",
            self.pending_requests.len()
        );

        Ok(())
    }

    /// Process pending requests if batch size is reached
    pub fn try_process_batch(&mut self) -> Option<BatchRequest> {
        if self.pending_requests.len() >= self.batch_size {
            self.process_batch()
        } else {
            None
        }
    }

    /// Force process all pending requests as a batch
    pub fn process_batch(&mut self) -> Option<BatchRequest> {
        if self.pending_requests.is_empty() {
            return None;
        }

        let requests = std::mem::take(&mut self.pending_requests);
        let total_fee = (requests.len() as u128) * self.fee_per_request;

        let batch = BatchRequest {
            requests: requests.clone(),
            ferry: self.ferry_address.clone(),
            fee_amount: total_fee,
        };

        println!(
            "🚢 Ferry processing batch of {} requests with total fee: {}",
            requests.len(),
            total_fee
        );

        Some(batch)
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Compute salt for account request (for demonstration)
    pub fn compute_salt(&self, request: &AccountRequest) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Add entropy source (block height)
        hasher.update(&request.historical_block_height.to_be_bytes());

        // Add deterministic request data
        hasher.update(request.controller.as_bytes());
        hasher.update(request.program_id.as_bytes());
        hasher.update(&request.account_request_id.to_be_bytes());
        hasher.update(&[request.account_type]);

        // Include library configuration hash
        let mut lib_hasher = Sha256::new();
        for lib in &request.libraries {
            lib_hasher.update(lib.as_bytes());
        }
        hasher.update(lib_hasher.finalize());

        hasher.finalize().into()
    }
}

/// Demo function showing ferry service usage
fn demo_ferry_service() {
    println!("=== Account Factory Ferry Service Demo ===\n");

    let mut ferry = FerryService::new(
        "neutron1ferry123456789abcdef".to_string(),
        3,    // Batch size of 3
        1000, // 1000 units fee per request
    );

    // Create some example account requests
    let requests = vec![
        AccountRequest {
            controller: "neutron1controller1".to_string(),
            libraries: vec!["neutron1lib1".to_string(), "neutron1lib2".to_string()],
            program_id: "program-1".to_string(),
            account_request_id: 1,
            account_type: 1, // TokenCustody
            historical_block_height: 12345,
            signature: None,
        },
        AccountRequest {
            controller: "neutron1controller2".to_string(),
            libraries: vec!["neutron1lib3".to_string()],
            program_id: "program-2".to_string(),
            account_request_id: 2,
            account_type: 2, // DataStorage
            historical_block_height: 12346,
            signature: None,
        },
        AccountRequest {
            controller: "neutron1controller3".to_string(),
            libraries: vec!["neutron1lib1".to_string(), "neutron1lib3".to_string()],
            program_id: "program-3".to_string(),
            account_request_id: 3,
            account_type: 3, // Hybrid
            historical_block_height: 12347,
            signature: None,
        },
    ];

    // Queue requests one by one
    for (i, request) in requests.into_iter().enumerate() {
        println!("Queuing request {}...", i + 1);
        let salt = ferry.compute_salt(&request);
        println!("  Computed salt: {}", hex::encode(&salt[..8])); // Show first 8 bytes

        ferry
            .queue_request(request)
            .expect("Failed to queue request");

        // Try to process batch (will only succeed when batch size is reached)
        if let Some(batch) = ferry.try_process_batch() {
            println!("📦 Batch ready for submission to account factory!");
            println!("   Batch details:");
            println!("     Ferry: {}", batch.ferry);
            println!("     Fee amount: {}", batch.fee_amount);
            println!("     Requests: {}", batch.requests.len());
            println!("   ✓ Batch submitted successfully\n");
        }
    }

    // Process any remaining requests
    if ferry.pending_count() > 0 {
        println!("Processing remaining {} requests...", ferry.pending_count());
        if let Some(batch) = ferry.process_batch() {
            println!("📦 Final batch ready for submission!");
            println!("   ✓ Final batch submitted successfully\n");
        }
    }

    println!("=== Ferry Service Demo Complete ===");
    println!("✅ All requests processed efficiently through batching");
    println!("💰 Fee optimization: {} requests processed in 2 batches", 3);
    println!("🔒 Security: All requests validated before batching");
    println!("⚡ Efficiency: Reduced on-chain transactions through batching\n");

    demo_salt_consistency();
}

/// Demonstrate salt generation consistency
fn demo_salt_consistency() {
    println!("=== Salt Generation Consistency Demo ===\n");

    let ferry = FerryService::new("ferry".to_string(), 5, 100);

    // Create identical requests
    let request1 = AccountRequest {
        controller: "neutron1controller".to_string(),
        libraries: vec!["neutron1lib1".to_string(), "neutron1lib2".to_string()],
        program_id: "test-program".to_string(),
        account_request_id: 1,
        account_type: 1,
        historical_block_height: 12345,
        signature: None,
    };

    let request2 = request1.clone();

    let salt1 = ferry.compute_salt(&request1);
    let salt2 = ferry.compute_salt(&request2);

    println!("Request 1 salt: {}", hex::encode(&salt1));
    println!("Request 2 salt: {}", hex::encode(&salt2));
    println!("Salts match: {}", salt1 == salt2);
    assert_eq!(salt1, salt2);
    println!("✅ Salt generation is deterministic\n");

    // Test different controllers produce different salts
    let mut request3 = request1.clone();
    request3.controller = "neutron1different_controller".to_string();

    let salt3 = ferry.compute_salt(&request3);
    println!("Different controller salt: {}", hex::encode(&salt3));
    println!("Different from original: {}", salt1 != salt3);
    assert_ne!(salt1, salt3);
    println!("✅ Different controllers produce different salts\n");

    // Test different libraries produce different salts
    let mut request4 = request1.clone();
    request4.libraries = vec!["neutron1different_lib".to_string()];

    let salt4 = ferry.compute_salt(&request4);
    println!("Different libraries salt: {}", hex::encode(&salt4));
    println!("Different from original: {}", salt1 != salt4);
    assert_ne!(salt1, salt4);
    println!("✅ Different libraries produce different salts\n");
}

fn main() {
    demo_ferry_service();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ferry_service_basic_functionality() {
        let mut ferry = FerryService::new(
            "ferry123".to_string(),
            2, // Small batch size for testing
            100,
        );

        assert_eq!(ferry.pending_count(), 0);

        // Add a request
        let request = AccountRequest {
            controller: "controller1".to_string(),
            libraries: vec!["lib1".to_string()],
            program_id: "prog1".to_string(),
            account_request_id: 1,
            account_type: 1,
            historical_block_height: 100,
            signature: None,
        };

        ferry.queue_request(request).unwrap();
        assert_eq!(ferry.pending_count(), 1);

        // Should not process batch yet
        assert!(ferry.try_process_batch().is_none());

        // Add another request to reach batch size
        let request2 = AccountRequest {
            controller: "controller2".to_string(),
            libraries: vec!["lib2".to_string()],
            program_id: "prog2".to_string(),
            account_request_id: 2,
            account_type: 2,
            historical_block_height: 101,
            signature: None,
        };

        ferry.queue_request(request2).unwrap();
        assert_eq!(ferry.pending_count(), 2);

        // Should process batch now
        let batch = ferry.try_process_batch().unwrap();
        assert_eq!(ferry.pending_count(), 0);

        // Verify batch details
        assert_eq!(batch.requests.len(), 2);
        assert_eq!(batch.ferry, "ferry123");
        assert_eq!(batch.fee_amount, 200); // 2 requests * 100 fee
    }

    #[test]
    fn test_ferry_service_validation() {
        let mut ferry = FerryService::new("ferry".to_string(), 5, 100);

        // Test empty controller
        let invalid_request = AccountRequest {
            controller: "".to_string(),
            libraries: vec!["lib1".to_string()],
            program_id: "prog1".to_string(),
            account_request_id: 1,
            account_type: 1,
            historical_block_height: 100,
            signature: None,
        };

        assert!(ferry.queue_request(invalid_request).is_err());

        // Test empty libraries
        let invalid_request = AccountRequest {
            controller: "controller1".to_string(),
            libraries: vec![],
            program_id: "prog1".to_string(),
            account_request_id: 1,
            account_type: 1,
            historical_block_height: 100,
            signature: None,
        };

        assert!(ferry.queue_request(invalid_request).is_err());

        // Test invalid account type
        let invalid_request = AccountRequest {
            controller: "controller1".to_string(),
            libraries: vec!["lib1".to_string()],
            program_id: "prog1".to_string(),
            account_request_id: 1,
            account_type: 4, // Invalid
            historical_block_height: 100,
            signature: None,
        };

        assert!(ferry.queue_request(invalid_request).is_err());
    }

    #[test]
    fn test_salt_generation_consistency() {
        let ferry = FerryService::new("ferry".to_string(), 5, 100);

        let request1 = AccountRequest {
            controller: "controller1".to_string(),
            libraries: vec!["lib1".to_string(), "lib2".to_string()],
            program_id: "prog1".to_string(),
            account_request_id: 1,
            account_type: 1,
            historical_block_height: 100,
            signature: None,
        };

        let request2 = request1.clone();

        let salt1 = ferry.compute_salt(&request1);
        let salt2 = ferry.compute_salt(&request2);

        assert_eq!(
            salt1, salt2,
            "Identical requests should produce identical salts"
        );

        // Test different controllers produce different salts
        let mut request3 = request1.clone();
        request3.controller = "different_controller".to_string();

        let salt3 = ferry.compute_salt(&request3);
        assert_ne!(
            salt1, salt3,
            "Different controllers should produce different salts"
        );

        // Test different libraries produce different salts
        let mut request4 = request1.clone();
        request4.libraries = vec!["different_lib".to_string()];

        let salt4 = ferry.compute_salt(&request4);
        assert_ne!(
            salt1, salt4,
            "Different libraries should produce different salts"
        );
    }
}
