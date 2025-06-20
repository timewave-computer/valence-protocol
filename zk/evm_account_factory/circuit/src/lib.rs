// Purpose: EVM Account Factory ZK Circuit for verifying salt generation integrity
#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Simple witness data for ZK proofs
#[derive(Debug, Clone)]
pub enum Witness {
    Data(Vec<u8>),
}

/// Input data for the EVM account factory circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitInput {
    /// Block hash used for entropy
    pub block_hash: [u8; 32],
    /// Controller address for account isolation
    pub controller: String,
    /// Program ID for the Valence program (string identifier)
    pub program_id: String,
    /// Account request ID for uniqueness
    pub account_request_id: u64,
    /// Hash of approved libraries
    pub libraries_hash: [u8; 32],
}

/// Public output of the circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitOutput {
    /// Generated salt for account creation
    pub salt: [u8; 32],
    /// Validation result
    pub is_valid: bool,
}

/// EVM account factory circuit implementation
pub struct EvmAccountFactoryCircuit;

impl EvmAccountFactoryCircuit {
    /// Execute the circuit with the given input
    pub fn execute(input: CircuitInput) -> CircuitOutput {
        // Generate deterministic salt - this is the security-critical operation
        // that must be proven in the ZK circuit
        let salt = Self::generate_salt(
            &input.block_hash,
            &input.controller,
            &input.program_id,
            input.account_request_id,
            &input.libraries_hash,
        );

        // Circuit always returns valid - all validation is done in the controller
        // The circuit's job is only to prove salt generation integrity
        let is_valid = true;

        CircuitOutput { salt, is_valid }
    }

    /// Generate deterministic salt for account creation
    fn generate_salt(
        block_hash: &[u8; 32],
        controller: &str,
        program_id: &str,
        account_request_id: u64,
        libraries_hash: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Add entropy sources in the same order as the contract
        hasher.update(block_hash);
        hasher.update(controller.as_bytes());
        hasher.update(program_id.as_bytes());
        hasher.update(account_request_id.to_le_bytes());
        hasher.update(libraries_hash);

        hasher.finalize().into()
    }
}

/// ZK VM circuit function for integration with the coprocessor
/// This function signature matches what the ZK coprocessor expects
pub fn circuit(witnesses: Vec<Witness>) -> Vec<u8> {
    // Extract witnesses in expected order
    assert!(witnesses.len() >= 5, "Expected at least 5 witnesses");

    let block_hash = match &witnesses[0] {
        Witness::Data(data) => {
            if data.len() >= 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&data[0..32]);
                hash
            } else {
                panic!("Invalid block hash size");
            }
        }
    };

    let libraries_hash = match &witnesses[1] {
        Witness::Data(data) => {
            if data.len() >= 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&data[0..32]);
                hash
            } else {
                panic!("Invalid libraries hash size");
            }
        }
    };

    let controller = match &witnesses[2] {
        Witness::Data(data) => String::from_utf8(data.clone()).unwrap_or_default(),
    };

    let program_id = match &witnesses[3] {
        Witness::Data(data) => String::from_utf8(data.clone()).unwrap_or_default(),
    };

    let account_request_id = match &witnesses[4] {
        Witness::Data(data) => {
            if data.len() >= 8 {
                u64::from_le_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ])
            } else {
                panic!("Invalid account request ID size");
            }
        }
    };

    // Create circuit input
    let input = CircuitInput {
        block_hash,
        controller,
        program_id,
        account_request_id,
        libraries_hash,
    };

    // Execute circuit
    let output = EvmAccountFactoryCircuit::execute(input);

    // Return serialized output: 32 bytes salt + 1 byte is_valid
    let mut result = Vec::with_capacity(33);
    result.extend_from_slice(&output.salt);
    result.push(if output.is_valid { 1 } else { 0 });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn create_test_input() -> CircuitInput {
        CircuitInput {
            block_hash: [2u8; 32],
            controller: String::from("test_controller"),
            libraries_hash: [0u8; 32],
            program_id: String::from("42"),
            account_request_id: 123,
        }
    }

    fn create_test_witnesses() -> Vec<Witness> {
        let block_hash = [2u8; 32];
        let libraries_hash = [0u8; 32];
        let controller = String::from("test_controller");
        let program_id = String::from("42");
        let account_request_id = 123u64;

        vec![
            Witness::Data(block_hash.to_vec()),
            Witness::Data(libraries_hash.to_vec()),
            Witness::Data(controller.into_bytes()),
            Witness::Data(program_id.into_bytes()),
            Witness::Data(account_request_id.to_le_bytes().to_vec()),
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
    #[should_panic(expected = "Expected at least 5 witnesses")]
    fn test_circuit_function_wrong_witness_count() {
        let witnesses = vec![Witness::Data(vec![1, 2, 3])]; // Wrong count
        circuit(witnesses);
    }

    #[test]
    fn test_public_outputs() {
        let input = create_test_input();
        let _output = EvmAccountFactoryCircuit::execute(input.clone());
        let public_data = circuit(vec![
            Witness::Data(input.block_hash.to_vec()),
            Witness::Data(input.libraries_hash.to_vec()),
            Witness::Data(input.controller.into_bytes()),
            Witness::Data(input.program_id.into_bytes()),
            Witness::Data(input.account_request_id.to_le_bytes().to_vec()),
        ]);

        // Should be 32 bytes for salt + 1 byte for is_valid flag
        assert_eq!(public_data.len(), 33);
        assert_eq!(public_data[32], 1); // is_valid should be true
    }
}
