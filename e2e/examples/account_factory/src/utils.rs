// Purpose: Utility functions for account factory e2e examples

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Compute a deterministic salt for account creation
pub fn compute_account_salt(
    controller: &str,
    program_id: &str,
    account_request_id: u64,
    libraries: &[String],
    historical_block_height: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    
    // Add entropy source (block height)
    hasher.update(&historical_block_height.to_be_bytes());
    
    // Add deterministic request data
    hasher.update(controller.as_bytes());
    hasher.update(program_id.as_bytes());
    hasher.update(&account_request_id.to_be_bytes());
    
    // Include library configuration hash
    // Sort libraries to ensure deterministic salt generation regardless of input order
    let mut sorted_libraries = libraries.to_vec();
    sorted_libraries.sort();
    let mut lib_hasher = Sha256::new();
    for lib in sorted_libraries {
        lib_hasher.update(lib.as_bytes());
    }
    hasher.update(lib_hasher.finalize());
    
    hasher.finalize().into()
}

/// Generate a mock signature for testing purposes
pub fn generate_mock_signature(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.update(&SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_be_bytes());
    hasher.finalize().to_vec()
}

/// Validate account request fields
pub fn validate_account_request_fields(
    controller: &str,
    program_id: &str,
    libraries: &[String],
) -> Result<()> {
    if controller.is_empty() {
        return Err(anyhow::anyhow!("Controller cannot be empty"));
    }
    if program_id.is_empty() {
        return Err(anyhow::anyhow!("Program ID cannot be empty"));
    }
    if libraries.is_empty() {
        return Err(anyhow::anyhow!("Libraries cannot be empty"));
    }
    Ok(())
}

/// Format duration for display
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs > 0 {
        format!("{}.{}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Generate a unique request ID based on current timestamp
pub fn generate_request_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

/// Truncate a hex string for display
pub fn truncate_hex(hex_str: &str, length: usize) -> String {
    if hex_str.len() <= length {
        hex_str.to_string()
    } else {
        format!("{}...", &hex_str[..length])
    }
} 