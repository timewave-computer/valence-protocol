// Purpose: EVM Account Factory ZK Circuit for verifying salt generation integrity
#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use valence_coprocessor::Witness;

/// Input for the ZK circuit (private)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitInput {
    /// Block hash used for entropy (private)
    pub block_hash: [u8; 32],
    /// Program ID for salt generation (private)
    pub program_id: u64,
    /// Account request ID for uniqueness (private)
    pub account_request_id: u64,
    /// Account type for differentiation (private)
    pub account_type: u8,
}

/// Output for the ZK circuit (public)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitOutput {
    /// The generated salt (public output)
    pub salt: [u8; 32],
    /// Whether salt generation succeeded
    pub is_valid: bool,
}

/// EVM Account Factory ZK Circuit
pub struct EvmAccountFactoryCircuit;

impl EvmAccountFactoryCircuit {
    /// Execute the ZK circuit - only proves salt generation integrity
    pub fn execute(input: CircuitInput) -> CircuitOutput {
        // Generate salt using entropy sources
        let salt = Self::generate_salt(
            &input.block_hash,
            input.program_id,
            input.account_request_id,
            input.account_type,
        );

        CircuitOutput {
            salt,
            is_valid: true, // Salt generation always succeeds if inputs are provided
        }
    }

    /// Generate deterministic salt - this is the core security property
    fn generate_salt(
        block_hash: &[u8; 32],
        program_id: u64,
        account_request_id: u64,
        account_type: u8,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(block_hash);
        hasher.update(&program_id.to_be_bytes());
        hasher.update(&account_request_id.to_be_bytes());
        hasher.update(&[account_type]);
        hasher.finalize().into()
    }

    /// Get public outputs for on-chain verification
    pub fn get_public_outputs(output: &CircuitOutput) -> Vec<u8> {
        let mut public_data = Vec::new();
        public_data.extend_from_slice(&output.salt);
        public_data.push(output.is_valid as u8);
        public_data
    }
}

/// Main circuit function for SP1 zkVM execution
/// 
/// This function is called by the SP1 zkVM and follows the valence-coprocessor pattern.
/// Witnesses are provided by the controller and contain the entropy data needed for salt generation.
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    // Ensure we have the expected number of witnesses
    assert_eq!(
        witnesses.len(),
        4,
        "Expected 4 witnesses: block_hash, program_id, account_request_id, account_type"
    );

    // Extract witness data
    let block_hash_bytes = witnesses[0].as_data().expect("Failed to get block hash");
    let program_id_bytes = witnesses[1].as_data().expect("Failed to get program ID");
    let account_request_id_bytes = witnesses[2].as_data().expect("Failed to get account request ID");
    let account_type_bytes = witnesses[3].as_data().expect("Failed to get account type");

    // Parse block hash
    let block_hash: [u8; 32] = <[u8; 32]>::try_from(block_hash_bytes)
        .expect("Block hash must be exactly 32 bytes");

    // Parse program ID
    let program_id = u64::from_le_bytes(
        <[u8; 8]>::try_from(program_id_bytes).expect("Program ID must be exactly 8 bytes"),
    );

    // Parse account request ID
    let account_request_id = u64::from_le_bytes(
        <[u8; 8]>::try_from(account_request_id_bytes).expect("Account request ID must be exactly 8 bytes"),
    );

    // Parse account type
    let account_type = account_type_bytes[0];

    // Validate account type
    assert!(
        matches!(account_type, 1 | 2 | 3),
        "Account type must be 1 (TokenCustody), 2 (DataStorage), or 3 (Hybrid)"
    );

    // Create circuit input
    let input = CircuitInput {
        block_hash,
        program_id,
        account_request_id,
        account_type,
    };

    // Execute the circuit
    let output = EvmAccountFactoryCircuit::execute(input);

    // Return the generated salt as public output
    EvmAccountFactoryCircuit::get_public_outputs(&output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use valence_coprocessor::Witness;
    use alloc::vec;

    fn create_test_input() -> CircuitInput {
        CircuitInput {
            block_hash: [2u8; 32],
            program_id: 42,
            account_request_id: 123,
            account_type: 1, // TokenCustody
        }
    }

    fn create_test_witnesses() -> Vec<Witness> {
        let block_hash = [2u8; 32];
        let program_id = 42u64;
        let account_request_id = 123u64;
        let account_type = 1u8;

        vec![
            Witness::Data(block_hash.to_vec()),
            Witness::Data(program_id.to_le_bytes().to_vec()),
            Witness::Data(account_request_id.to_le_bytes().to_vec()),
            Witness::Data(vec![account_type]),
        ]
    }

    #[test]
    fn test_salt_generation() {
        let input = create_test_input();
        let output = EvmAccountFactoryCircuit::execute(input.clone());
        
        assert!(output.is_valid);
        assert_ne!(output.salt, [0u8; 32]);

        // Same inputs should produce same salt
        let output2 = EvmAccountFactoryCircuit::execute(input);
        assert_eq!(output.salt, output2.salt);
    }

    #[test]
    fn test_salt_differentiation() {
        let mut input1 = create_test_input();
        let mut input2 = create_test_input();
        
        // Different account types should produce different salts
        input1.account_type = 1;
        input2.account_type = 2;

        let output1 = EvmAccountFactoryCircuit::execute(input1);
        let output2 = EvmAccountFactoryCircuit::execute(input2);

        assert_ne!(output1.salt, output2.salt);
    }

    #[test]
    fn test_entropy_sensitivity() {
        let mut input1 = create_test_input();
        let mut input2 = create_test_input();
        
        // Different entropy should produce different salts
        input1.block_hash = [1u8; 32];
        input2.block_hash = [2u8; 32];

        let output1 = EvmAccountFactoryCircuit::execute(input1);
        let output2 = EvmAccountFactoryCircuit::execute(input2);

        assert_ne!(output1.salt, output2.salt);
    }

    #[test]
    fn test_circuit_function_with_witnesses() {
        let witnesses = create_test_witnesses();
        let public_outputs = circuit(witnesses);
        
        // Should return 33 bytes: 32 bytes salt + 1 byte is_valid
        assert_eq!(public_outputs.len(), 33);
        
        // Last byte should be 1 (true for is_valid)
        assert_eq!(public_outputs[32], 1);
        
        // Salt should not be all zeros
        assert_ne!(&public_outputs[0..32], &[0u8; 32]);
    }

    #[test]
    #[should_panic(expected = "Expected 4 witnesses")]
    fn test_circuit_function_wrong_witness_count() {
        let witnesses = vec![Witness::Data(vec![1, 2, 3])]; // Wrong count
        circuit(witnesses);
    }

    #[test]
    #[should_panic(expected = "Account type must be 1")]
    fn test_circuit_function_invalid_account_type() {
        let mut witnesses = create_test_witnesses();
        witnesses[3] = Witness::Data(vec![4]); // Invalid account type
        circuit(witnesses);
    }

    #[test]
    fn test_public_outputs() {
        let input = create_test_input();
        let output = EvmAccountFactoryCircuit::execute(input);
        
        let public_data = EvmAccountFactoryCircuit::get_public_outputs(&output);
        assert_eq!(public_data.len(), 33); // 32 bytes salt + 1 byte is_valid
        assert_eq!(&public_data[0..32], &output.salt);
        assert_eq!(public_data[32], output.is_valid as u8);
    }
} 