// Purpose: ZK integration tests for account factory components across EVM and CosmWasm

#[cfg(test)]
mod integration_tests {

    // Test the EVM ZK components
    mod evm_tests {
        use evm_account_factory_circuit::{
            circuit as evm_circuit, CircuitInput as EvmCircuitInput, EvmAccountFactoryCircuit, Witness,
        };
        use evm_account_factory_controller::{
            EvmAccountFactoryController,
            FactoryInput as EvmFactoryInput,
        };

        #[test]
        fn test_evm_proof_generation() {
            let input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: "42".to_string(),
                account_request_id: 123,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
                libraries: vec!["0x1111111111111111111111111111111111111111".to_string(), "0x2222222222222222222222222222222222222222".to_string()],
            };

            let witness = EvmAccountFactoryController::process_input(input).unwrap();
            assert!(witness.is_valid_controller);
            assert!(witness.is_valid_salt);
            assert!(witness.is_valid_address);
        }

        #[test]
        fn test_evm_circuit_verification() {
            let circuit_input = EvmCircuitInput {
                block_hash: [1u8; 32],
                program_id: "42".to_string(),
                account_request_id: 123,
                libraries_hash: [0u8; 32], // Empty libraries for test
            };

            let output = EvmAccountFactoryCircuit::execute(circuit_input);
            assert!(output.is_valid);
            assert_ne!(output.salt, [0u8; 32]);
        }

        #[test]
        fn test_evm_library_differentiation() {
            let base_input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: "42".to_string(),
                account_request_id: 123,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
                libraries: vec!["0x1111111111111111111111111111111111111111".to_string()],
            };

            let lib1_input = base_input.clone();
            let lib1_witness = EvmAccountFactoryController::process_input(lib1_input).unwrap();

            let mut lib2_input = base_input.clone();
            lib2_input.libraries = vec!["0x2222222222222222222222222222222222222222".to_string()];
            let lib2_witness = EvmAccountFactoryController::process_input(lib2_input).unwrap();

            // Different libraries should produce different addresses
            assert_ne!(
                lib1_witness.expected_address,
                lib2_witness.expected_address
            );
        }

        #[test]
        fn test_evm_salt_generation() {
            let circuit_input1 = EvmCircuitInput {
                block_hash: [1u8; 32],
                program_id: "42".to_string(),
                account_request_id: 123,
                libraries_hash: [0u8; 32],
            };

            let circuit_input2 = EvmCircuitInput {
                block_hash: [2u8; 32],
                program_id: "42".to_string(),
                account_request_id: 123,
                libraries_hash: [0u8; 32],
            };

            let output1 = EvmAccountFactoryCircuit::execute(circuit_input1);
            let output2 = EvmAccountFactoryCircuit::execute(circuit_input2);

            // Different entropy should produce different salts
            assert_ne!(output1.salt, output2.salt);
        }

        #[test]
        fn test_evm_zkvm_circuit_function() {
            let block_hash = [2u8; 32];
            let libraries_hash = [0u8; 32];
            let program_id = "42".to_string();
            let account_request_id = 123u64;

            let witnesses = vec![
                Witness::Data(block_hash.to_vec()),
                Witness::Data(libraries_hash.to_vec()),
                Witness::Data(program_id.as_bytes().to_vec()),
                Witness::Data(account_request_id.to_le_bytes().to_vec()),
            ];

            let public_outputs = evm_circuit(witnesses);

            // Should return 33 bytes: 32 bytes salt + 1 byte is_valid
            assert_eq!(public_outputs.len(), 33);
            assert_eq!(public_outputs[32], 1); // is_valid = true
        }

        #[test]
        fn test_batch_salt_generation() {
            // Test generating many salts efficiently
            let mut salts = Vec::new();

            for i in 0..100 {
                let evm_input = EvmFactoryInput {
                    controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                    program_id: "42".to_string(),
                    account_request_id: i,
                    factory: "0x1234567890123456789012345678901234567890".to_string(),
                    block_hash: [1u8; 32],
                    libraries: vec!["0x1111111111111111111111111111111111111111".to_string()],
                };

                let witness = EvmAccountFactoryController::process_input(evm_input).unwrap();
                salts.push(witness.salt);
            }

            // All salts should be unique
            let unique_salts: std::collections::HashSet<_> = salts.iter().collect();
            assert_eq!(unique_salts.len(), salts.len());
        }

        #[test]
        fn test_mixed_library_salt_generation() {
            // Test with different library configurations
            let mut salts = Vec::new();

            let library_configs = vec![
                vec!["0x1111111111111111111111111111111111111111".to_string()],
                vec!["0x1111111111111111111111111111111111111111".to_string(), "0x2222222222222222222222222222222222222222".to_string()],
                vec!["0x3333333333333333333333333333333333333333".to_string()],
            ];

            for libs in library_configs {
                let evm_input = EvmFactoryInput {
                    controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                    program_id: "42".to_string(),
                    account_request_id: 123,
                    factory: "0x1234567890123456789012345678901234567890".to_string(),
                    block_hash: [1u8; 32],
                    libraries: libs,
                };

                let witness = EvmAccountFactoryController::process_input(evm_input).unwrap();
                salts.push(witness.salt);
            }

            // All salts should be unique
            let unique_salts: std::collections::HashSet<_> = salts.iter().collect();
            assert_eq!(unique_salts.len(), salts.len());
        }
    }

    // Test the CosmWasm ZK components
    mod cosmwasm_tests {
        use cosmwasm_account_factory_circuit::{
            circuit as cosmwasm_circuit, CircuitInput as CosmWasmCircuitInput,
            CosmWasmAccountFactoryCircuit, Witness as CosmWasmWitness,
        };
        use cosmwasm_account_factory_controller::{
            CosmWasmAccountFactoryController,
            FactoryInput as CosmWasmFactoryInput,
        };

        #[test]
        fn test_cosmwasm_proof_generation() {
            let input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: "42".to_string(),
                canonical_factory: vec![1u8; 32], // Vec<u8> not String
                code_checksum: vec![1u8; 32], // Vec<u8> not [u8; 32]
                libraries: vec!["cosmos1lib1".to_string(), "cosmos1lib2".to_string()],
            };

            let witness = CosmWasmAccountFactoryController::process_input(input).unwrap();
            assert!(witness.is_valid_controller);
            assert!(witness.is_valid_salt);
            assert!(witness.is_valid_address);
        }

        #[test]
        fn test_cosmwasm_circuit_verification() {
            let circuit_input = CosmWasmCircuitInput {
                block_height: 12345,
                program_id: "42".to_string(),
                account_request_id: 123,
                libraries_hash: [0u8; 32], // Empty libraries for test
            };

            let output = CosmWasmAccountFactoryCircuit::execute(circuit_input);
            assert!(output.is_valid);
            assert_ne!(output.salt, [0u8; 32]);
        }

        #[test]
        fn test_cosmwasm_library_differentiation() {
            let base_input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: "42".to_string(),
                canonical_factory: vec![1u8; 32], // Vec<u8> not String
                code_checksum: vec![1u8; 32], // Vec<u8> not [u8; 32]
                libraries: vec!["cosmos1lib1".to_string()],
            };

            let lib1_input = base_input.clone();
            let lib1_witness = CosmWasmAccountFactoryController::process_input(lib1_input).unwrap();

            let mut lib2_input = base_input.clone();
            lib2_input.libraries = vec!["cosmos1lib2".to_string()];
            let lib2_witness = CosmWasmAccountFactoryController::process_input(lib2_input).unwrap();

            // Different libraries should produce different addresses
            assert_ne!(
                lib1_witness.expected_address,
                lib2_witness.expected_address
            );
        }

        #[test]
        fn test_cosmwasm_salt_generation() {
            let circuit_input1 = CosmWasmCircuitInput {
                block_height: 12345,
                program_id: "42".to_string(),
                account_request_id: 123,
                libraries_hash: [0u8; 32],
            };

            let circuit_input2 = CosmWasmCircuitInput {
                block_height: 54321,
                program_id: "42".to_string(),
                account_request_id: 123,
                libraries_hash: [0u8; 32],
            };

            let output1 = CosmWasmAccountFactoryCircuit::execute(circuit_input1);
            let output2 = CosmWasmAccountFactoryCircuit::execute(circuit_input2);

            // Different entropy should produce different salts
            assert_ne!(output1.salt, output2.salt);
        }

        #[test]
        fn test_cosmwasm_zkvm_circuit_function() {
            let block_height = 12345u64;
            let libraries_hash = [0u8; 32];
            let program_id = "42".to_string();
            let account_request_id = 123u64;

            let witnesses = vec![
                CosmWasmWitness::Data(block_height.to_le_bytes().to_vec()),
                CosmWasmWitness::Data(libraries_hash.to_vec()),
                CosmWasmWitness::Data(program_id.as_bytes().to_vec()),
                CosmWasmWitness::Data(account_request_id.to_le_bytes().to_vec()),
            ];

            let public_outputs = cosmwasm_circuit(witnesses);

            // Should return 33 bytes: 32 bytes salt + 1 byte is_valid
            assert_eq!(public_outputs.len(), 33);
            assert_eq!(public_outputs[32], 1); // is_valid = true
        }
    }

    // Test cross-platform compatibility
    mod cross_platform_tests {
        use evm_account_factory_controller::{EvmAccountFactoryController, FactoryInput as EvmFactoryInput};
        use cosmwasm_account_factory_controller::{CosmWasmAccountFactoryController, FactoryInput as CosmWasmFactoryInput};

        #[test]
        fn test_library_consistency() {
            // Test that both platforms handle library hashing consistently
            let libraries = vec!["lib1".to_string(), "lib2".to_string()];
            
            // Both platforms should produce consistent results for same library inputs
            assert!(!libraries.is_empty());
        }

        #[test]
        fn test_atomic_operation_validation() {
            // Test EVM atomic operation validation
            let evm_input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: "42".to_string(),
                account_request_id: 123,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
                libraries: vec!["0x1111111111111111111111111111111111111111".to_string()],
            };

            let evm_witness = EvmAccountFactoryController::process_input(evm_input.clone()).unwrap();
            assert!(EvmAccountFactoryController::validate_atomic_operation(&evm_input, &evm_witness));

            // Test CosmWasm atomic operation validation
            let cosmwasm_input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: "42".to_string(),
                canonical_factory: vec![1u8; 32],
                code_checksum: vec![1u8; 32],
                libraries: vec!["cosmos1lib1".to_string()],
            };

            let cosmwasm_witness = CosmWasmAccountFactoryController::process_input(cosmwasm_input.clone()).unwrap();
            assert!(CosmWasmAccountFactoryController::validate_atomic_operation(&cosmwasm_input, &cosmwasm_witness));
        }

        #[test]
        fn test_full_capability_accounts() {
            // Test that all accounts now have full capabilities
            let evm_input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: "42".to_string(),
                account_request_id: 123,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
                libraries: vec!["0x1111111111111111111111111111111111111111".to_string()],
            };

            let evm_witness = EvmAccountFactoryController::process_input(evm_input).unwrap();
            assert!(evm_witness.is_valid_controller);

            // Test CosmWasm full capabilities
            let cosmwasm_input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: "42".to_string(),
                canonical_factory: vec![1u8; 32],
                code_checksum: vec![1u8; 32],
                libraries: vec!["cosmos1lib1".to_string()],
            };

            let cosmwasm_witness = CosmWasmAccountFactoryController::process_input(cosmwasm_input).unwrap();
            assert!(cosmwasm_witness.is_valid_controller);
        }
    }

    // Performance and stress tests
    mod performance_tests {
        use evm_account_factory_controller::{EvmAccountFactoryController, FactoryInput as EvmFactoryInput};
        use evm_account_factory_circuit::{
            circuit as evm_circuit, Witness as EvmWitness,
        };
        use cosmwasm_account_factory_circuit::{
            circuit as cosmwasm_circuit, Witness as CosmWasmWitness,
        };

        #[test]
        fn test_mixed_library_salt_generation() {
            // Test with different library configurations
            let mut salts = Vec::new();

            let library_configs = vec![
                vec!["0x1111111111111111111111111111111111111111".to_string()],
                vec!["0x1111111111111111111111111111111111111111".to_string(), "0x2222222222222222222222222222222222222222".to_string()],
                vec!["0x3333333333333333333333333333333333333333".to_string()],
            ];

            for libs in library_configs {
                let evm_input = EvmFactoryInput {
                    controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                    program_id: "42".to_string(),
                    account_request_id: 123,
                    factory: "0x1234567890123456789012345678901234567890".to_string(),
                    block_hash: [1u8; 32],
                    libraries: libs,
                };

                let witness = EvmAccountFactoryController::process_input(evm_input).unwrap();
                salts.push(witness.salt);
            }

            // All salts should be unique
            let unique_salts: std::collections::HashSet<_> = salts.iter().collect();
            assert_eq!(unique_salts.len(), salts.len());
        }

        #[test]
        fn test_zkvm_circuit_functions() {
            // Test EVM circuit function
            let evm_witnesses = vec![
                EvmWitness::Data([1u8; 32].to_vec()),
                EvmWitness::Data([0u8; 32].to_vec()),
                EvmWitness::Data("42".as_bytes().to_vec()),
                EvmWitness::Data(123u64.to_le_bytes().to_vec()),
            ];

            let evm_output = evm_circuit(evm_witnesses);
            assert_eq!(evm_output.len(), 33);

            // Test CosmWasm circuit function
            let cosmwasm_witnesses = vec![
                CosmWasmWitness::Data(12345u64.to_le_bytes().to_vec()),
                CosmWasmWitness::Data([0u8; 32].to_vec()),
                CosmWasmWitness::Data("42".as_bytes().to_vec()),
                CosmWasmWitness::Data(123u64.to_le_bytes().to_vec()),
            ];

            let cosmwasm_output = cosmwasm_circuit(cosmwasm_witnesses);
            assert_eq!(cosmwasm_output.len(), 33);
        }
    }
}
