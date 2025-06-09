// Purpose: Utility functions for Account Factory E2E tests

use std::time::Duration;
use sha2::{Digest, Sha256};

use crate::AccountCreationRequest;

/// Compute deterministic salt for account creation
pub fn compute_salt(request: &AccountCreationRequest) -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    // Add request components to salt
    hasher.update(request.controller.as_bytes());
    hasher.update(request.program_id.as_bytes());
    hasher.update(request.account_request_id.to_be_bytes());
    hasher.update([request.account_type]);
    
    // Add libraries hash
    let mut lib_hasher = Sha256::new();
    for lib in &request.libraries {
        lib_hasher.update(lib.as_bytes());
    }
    hasher.update(lib_hasher.finalize());
    
    hasher.finalize().into()
}

/// Validate account creation request
pub fn validate_request(request: &AccountCreationRequest) -> Result<(), String> {
    if request.controller.is_empty() {
        return Err("Controller cannot be empty".to_string());
    }
    
    if request.program_id.is_empty() {
        return Err("Program ID cannot be empty".to_string());
    }
    
    if !matches!(request.account_type, 1 | 2 | 3) {
        return Err("Invalid account type".to_string());
    }
    
    Ok(())
}

/// Generate test request with unique parameters
pub fn generate_test_request(base_request_id: u64, account_type: u8) -> AccountCreationRequest {
    AccountCreationRequest {
        controller: "0x742d35Cc6634C0532925a3b8D698B6CDb4fdC5C8".to_string(),
        program_id: format!("test_program_{}", base_request_id),
        account_request_id: base_request_id,
        account_type,
        libraries: vec![],
        historical_block_number: None,
        target_chain: None,
    }
}

/// Sleep for given duration (async)
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Retry operation with exponential backoff
pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut delay = initial_delay;
    
    for _ in 0..max_retries {
        match operation() {
            Ok(result) => return Ok(result),
            Err(err) => {
                if delay.as_secs() >= 60 {
                    return Err(err);
                }
                sleep(delay).await;
                delay *= 2;
            }
        }
    }
    
    operation()
}

/// Format duration for display
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs > 0 {
        format!("{}s {}ms", secs, millis)
    } else {
        format!("{}ms", millis)
    }
} 