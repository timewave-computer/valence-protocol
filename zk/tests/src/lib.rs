// Purpose: ZK integration tests for account factory components across EVM and CosmWasm

#[cfg(test)]
mod integration_tests {

    // Test the EVM ZK components
    mod evm_tests {
        use evm_account_factory_controller::{
            AccountType as EvmAccountType, EvmAccountFactoryController, FactoryInput as EvmFactoryInput,
        };
        use evm_account_factory_circuit::{
            CircuitInput as EvmCircuitInput, EvmAccountFactoryCircuit, circuit as evm_circuit,
        };
        use valence_coprocessor::Witness;

        #[test]
        fn test_evm_proof_generation() {
            let input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: 42,
                account_request_id: 123,
                account_type: EvmAccountType::TokenCustody,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
            };

            let witness = EvmAccountFactoryController::process_input(input).unwrap();
            assert!(witness.is_valid_controller);
            assert!(witness.is_valid_salt);
            assert!(witness.is_valid_address);
        }

        #[test]
        fn test_evm_circuit_verification() {
            let circuit_input = EvmCircuitInput {
                block_hash: [2u8; 32],
                program_id: 42,
                account_request_id: 123,
                account_type: 1, // TokenCustody
            };

            let output = EvmAccountFactoryCircuit::execute(circuit_input);
            assert!(output.is_valid);
            assert_ne!(output.salt, [0u8; 32]);
        }

        #[test]
        fn test_evm_account_type_differentiation() {
            let base_input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: 42,
                account_request_id: 123,
                account_type: EvmAccountType::TokenCustody,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
            };

            let mut token_input = base_input.clone();
            token_input.account_type = EvmAccountType::TokenCustody;
            let token_witness = EvmAccountFactoryController::process_input(token_input).unwrap();

            let mut data_input = base_input.clone();
            data_input.account_type = EvmAccountType::DataStorage;
            let data_witness = EvmAccountFactoryController::process_input(data_input).unwrap();

            // Different account types should produce different addresses
            assert_ne!(token_witness.expected_address, data_witness.expected_address);
        }

        #[test]
        fn test_evm_salt_generation() {
            let circuit_input1 = EvmCircuitInput {
                block_hash: [1u8; 32],
                program_id: 42,
                account_request_id: 123,
                account_type: 1,
            };

            let circuit_input2 = EvmCircuitInput {
                block_hash: [2u8; 32],
                program_id: 42,
                account_request_id: 123,
                account_type: 1,
            };

            let output1 = EvmAccountFactoryCircuit::execute(circuit_input1);
            let output2 = EvmAccountFactoryCircuit::execute(circuit_input2);

            // Different entropy should produce different salts
            assert_ne!(output1.salt, output2.salt);
        }

        #[test]
        fn test_evm_zkvm_circuit_function() {
            let block_hash = [2u8; 32];
            let program_id = 42u64;
            let account_request_id = 123u64;
            let account_type = 1u8;

            let witnesses = vec![
                Witness::Data(block_hash.to_vec()),
                Witness::Data(program_id.to_le_bytes().to_vec()),
                Witness::Data(account_request_id.to_le_bytes().to_vec()),
                Witness::Data(vec![account_type]),
            ];

            let public_outputs = evm_circuit(witnesses);
            
            // Should return 33 bytes: 32 bytes salt + 1 byte is_valid
            assert_eq!(public_outputs.len(), 33);
            assert_eq!(public_outputs[32], 1); // is_valid = true
        }
    }

    // Test the CosmWasm ZK components
    mod cosmwasm_tests {
        use cosmwasm_account_factory_controller::{
            AccountType as CosmWasmAccountType, CosmWasmAccountFactoryController, FactoryInput as CosmWasmFactoryInput,
        };
        use cosmwasm_account_factory_circuit::{
            CircuitInput as CosmWasmCircuitInput, CosmWasmAccountFactoryCircuit, circuit as cosmwasm_circuit,
        };
        use valence_coprocessor::Witness;

        #[test]
        fn test_cosmwasm_proof_generation() {
            let input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                account_type: CosmWasmAccountType::DataStorage,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: 42,
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
                program_id: 42,
                account_request_id: 123,
                account_type: 2, // DataStorage
            };

            let output = CosmWasmAccountFactoryCircuit::execute(circuit_input);
            assert!(output.is_valid);
            assert_ne!(output.salt, [0u8; 32]);
        }

        #[test]
        fn test_cosmwasm_account_type_differentiation() {
            let base_input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                account_type: CosmWasmAccountType::TokenCustody,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: 42,
            };

            let mut token_input = base_input.clone();
            token_input.account_type = CosmWasmAccountType::TokenCustody;
            let token_witness = CosmWasmAccountFactoryController::process_input(token_input).unwrap();

            let mut hybrid_input = base_input.clone();
            hybrid_input.account_type = CosmWasmAccountType::Hybrid;
            let hybrid_witness = CosmWasmAccountFactoryController::process_input(hybrid_input).unwrap();

            // Different account types should produce different addresses
            assert_ne!(token_witness.expected_address, hybrid_witness.expected_address);
        }

        #[test]
        fn test_cosmwasm_salt_generation() {
            let circuit_input1 = CosmWasmCircuitInput {
                block_height: 12345,
                program_id: 42,
                account_request_id: 123,
                account_type: 1,
            };

            let circuit_input2 = CosmWasmCircuitInput {
                block_height: 54321,
                program_id: 42,
                account_request_id: 123,
                account_type: 1,
            };

            let output1 = CosmWasmAccountFactoryCircuit::execute(circuit_input1);
            let output2 = CosmWasmAccountFactoryCircuit::execute(circuit_input2);

            // Different entropy should produce different salts
            assert_ne!(output1.salt, output2.salt);
        }

        #[test]
        fn test_cosmwasm_zkvm_circuit_function() {
            let block_height = 12345u64;
            let program_id = 42u64;
            let account_request_id = 123u64;
            let account_type = 1u8;

            let witnesses = vec![
                Witness::Data(block_height.to_le_bytes().to_vec()),
                Witness::Data(program_id.to_le_bytes().to_vec()),
                Witness::Data(account_request_id.to_le_bytes().to_vec()),
                Witness::Data(vec![account_type]),
            ];

            let public_outputs = cosmwasm_circuit(witnesses);
            
            // Should return 33 bytes: 32 bytes salt + 1 byte is_valid
            assert_eq!(public_outputs.len(), 33);
            assert_eq!(public_outputs[32], 1); // is_valid = true
        }
    }

    // Cross-platform integration tests
    mod cross_platform_tests {
        use evm_account_factory_controller::{
            AccountType as EvmAccountType, EvmAccountFactoryController, FactoryInput as EvmFactoryInput,
        };
        use cosmwasm_account_factory_controller::{
            AccountType as CosmWasmAccountType, CosmWasmAccountFactoryController, FactoryInput as CosmWasmFactoryInput,
        };

        #[test]
        fn test_account_type_consistency() {
            // Test that account types behave consistently across platforms
            let evm_types = vec![
                EvmAccountType::TokenCustody,
                EvmAccountType::DataStorage,
                EvmAccountType::Hybrid,
            ];

            let cosmwasm_types = vec![
                CosmWasmAccountType::TokenCustody,
                CosmWasmAccountType::DataStorage,
                CosmWasmAccountType::Hybrid,
            ];

            // Both platforms should support the same account types
            assert_eq!(evm_types.len(), cosmwasm_types.len());

            // Test byte representation consistency
            for (evm_type, cosmwasm_type) in evm_types.iter().zip(cosmwasm_types.iter()) {
                assert_eq!(evm_type.to_byte(), cosmwasm_type.to_byte());
            }
        }

        #[test]
        fn test_atomic_operation_validation() {
            // Test EVM atomic operations
            let evm_input = EvmFactoryInput {
                controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
                program_id: 42,
                account_request_id: 123,
                account_type: EvmAccountType::Hybrid,
                factory: "0x1234567890123456789012345678901234567890".to_string(),
                block_hash: [1u8; 32],
            };

            let evm_witness = EvmAccountFactoryController::process_input(evm_input.clone()).unwrap();
            assert!(EvmAccountFactoryController::validate_atomic_operation(
                &evm_input, &evm_witness
            ));

            // Test CosmWasm atomic operations
            let cosmwasm_input = CosmWasmFactoryInput {
                controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
                code_id: 42,
                account_request_id: 123,
                account_type: CosmWasmAccountType::Hybrid,
                factory: "cosmos1factory123456789abcdef0123456789abcdef01".to_string(),
                block_height: 12345,
                program_id: 42,
            };

            let cosmwasm_witness = CosmWasmAccountFactoryController::process_input(cosmwasm_input.clone()).unwrap();
            assert!(CosmWasmAccountFactoryController::validate_atomic_operation(
                &cosmwasm_input, &cosmwasm_witness
            ));
        }

        #[test]
        fn test_account_capability_configuration() {
            // Test EVM account capability configuration
            let mut evm_init_msg = serde_json::json!({});
            
            EvmAccountFactoryController::process_account_capabilities(
                &EvmAccountType::TokenCustody,
                &mut evm_init_msg,
            ).unwrap();
            assert_eq!(evm_init_msg["enable_token_custody"], true);
            assert_eq!(evm_init_msg["enable_data_storage"], false);

            EvmAccountFactoryController::process_account_capabilities(
                &EvmAccountType::Hybrid,
                &mut evm_init_msg,
            ).unwrap();
            assert_eq!(evm_init_msg["enable_token_custody"], true);
            assert_eq!(evm_init_msg["enable_data_storage"], true);

            // Test CosmWasm account capability configuration
            let mut cosmwasm_init_msg = serde_json::json!({});
            
            CosmWasmAccountFactoryController::process_account_capabilities(
                &CosmWasmAccountType::DataStorage,
                &mut cosmwasm_init_msg,
            ).unwrap();
            assert_eq!(cosmwasm_init_msg["enable_token_custody"], false);
            assert_eq!(cosmwasm_init_msg["enable_data_storage"], true);

            CosmWasmAccountFactoryController::process_account_capabilities(
                &CosmWasmAccountType::Hybrid,
                &mut cosmwasm_init_msg,
            ).unwrap();
            assert_eq!(cosmwasm_init_msg["enable_token_custody"], true);
            assert_eq!(cosmwasm_init_msg["enable_data_storage"], true);
        }
    }

    // Performance and batch testing
    mod performance_tests {
        use evm_account_factory_circuit::{EvmAccountFactoryCircuit, CircuitInput as EvmCircuitInput, circuit as evm_circuit};
        use cosmwasm_account_factory_circuit::{CosmWasmAccountFactoryCircuit, CircuitInput as CosmWasmCircuitInput, circuit as cosmwasm_circuit};
        use valence_coprocessor::Witness;

        #[test]
        fn test_batch_salt_generation() {
            // Test multiple salt generations to simulate batch processing
            for i in 0..5 {
                let evm_input = EvmCircuitInput {
                    block_hash: [1u8; 32],
                    program_id: 42,
                    account_request_id: i as u64,
                    account_type: 1,
                };

                let evm_output = EvmAccountFactoryCircuit::execute(evm_input);
                assert!(evm_output.is_valid);
                assert_ne!(evm_output.salt, [0u8; 32]);

                let cosmwasm_input = CosmWasmCircuitInput {
                    block_height: 12345,
                    program_id: 42,
                    account_request_id: i as u64,
                    account_type: 2,
                };

                let cosmwasm_output = CosmWasmAccountFactoryCircuit::execute(cosmwasm_input);
                assert!(cosmwasm_output.is_valid);
                assert_ne!(cosmwasm_output.salt, [0u8; 32]);
            }
        }

        #[test]
        fn test_mixed_account_type_salt_generation() {
            let account_types = vec![1, 2, 3]; // TokenCustody, DataStorage, Hybrid

            for (i, account_type) in account_types.into_iter().enumerate() {
                let evm_input = EvmCircuitInput {
                    block_hash: [1u8; 32],
                    program_id: 42,
                    account_request_id: i as u64,
                    account_type,
                };

                let evm_output = EvmAccountFactoryCircuit::execute(evm_input);
                assert!(evm_output.is_valid);

                let cosmwasm_input = CosmWasmCircuitInput {
                    block_height: 12345,
                    program_id: 42,
                    account_request_id: i as u64,
                    account_type,
                };

                let cosmwasm_output = CosmWasmAccountFactoryCircuit::execute(cosmwasm_input);
                assert!(cosmwasm_output.is_valid);
            }
        }

        #[test]
        fn test_zkvm_circuit_functions() {
            // Test EVM zkVM circuit function
            let evm_witnesses = vec![
                Witness::Data([1u8; 32].to_vec()),          // block_hash
                Witness::Data(42u64.to_le_bytes().to_vec()), // program_id
                Witness::Data(123u64.to_le_bytes().to_vec()), // account_request_id
                Witness::Data(vec![1u8]),                     // account_type
            ];

            let evm_output = evm_circuit(evm_witnesses);
            assert_eq!(evm_output.len(), 33);
            assert_eq!(evm_output[32], 1); // is_valid = true

            // Test CosmWasm zkVM circuit function
            let cosmwasm_witnesses = vec![
                Witness::Data(12345u64.to_le_bytes().to_vec()), // block_height
                Witness::Data(42u64.to_le_bytes().to_vec()),     // program_id
                Witness::Data(123u64.to_le_bytes().to_vec()),    // account_request_id
                Witness::Data(vec![1u8]),                        // account_type
            ];

            let cosmwasm_output = cosmwasm_circuit(cosmwasm_witnesses);
            assert_eq!(cosmwasm_output.len(), 33);
            assert_eq!(cosmwasm_output[32], 1); // is_valid = true
        }
    }
} 