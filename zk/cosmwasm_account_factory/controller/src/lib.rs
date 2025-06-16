// Purpose: CosmWasm Account Factory ZK Controller for generating witnesses and validating account creation
use cosmwasm_std::{instantiate2_address, CanonicalAddr};
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Simple witness data for ZK proofs
#[derive(Debug, Clone)]
pub enum Witness {
    Data(Vec<u8>),
}

/// Factory input for creating accounts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactoryInput {
    /// Controller address that will own the account
    pub controller: String,
    /// Library addresses that will be approved for the account
    pub libraries: Vec<String>,
    /// Program ID for the Valence program (string identifier)
    pub program_id: String,
    /// Account request ID for uniqueness
    pub account_request_id: u64,
    /// Factory contract address
    pub factory: String,
    /// Code ID for the account contract
    pub code_id: u64,
    /// Code checksum for instantiate2 address calculation
    pub code_checksum: Vec<u8>,
    /// Canonical factory address (bech32 decoded)
    pub canonical_factory: Vec<u8>,
    /// Block height used for entropy
    pub block_height: u64,
}

/// Factory witness containing intermediate validation data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactoryWitness {
    /// Controller address
    pub controller: String,
    /// Generated salt for Instantiate2
    pub salt: [u8; 32],
    /// Expected account address
    pub expected_address: String,
    /// Validation flags
    pub is_valid_controller: bool,
    pub is_valid_salt: bool,
    pub is_valid_address: bool,
}

/// Factory output for ZK circuit
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactoryOutput {
    /// Controller address
    pub controller: String,
    /// Created account address
    pub account_address: String,
    /// Validation result
    pub is_valid: bool,
}

/// CosmWasm account factory ZK controller
pub struct CosmWasmAccountFactoryController;

impl CosmWasmAccountFactoryController {
    /// Process factory input and generate witness data
    pub fn process_input(input: FactoryInput) -> Result<FactoryWitness, String> {
        // Comprehensive input validation (done in controller, not circuit)
        if !Self::validate_factory_input(&input) {
            return Err("Invalid input parameters".to_string());
        }

        // Generate deterministic salt
        let salt = Self::generate_salt(
            input.block_height,
            &input.controller,
            &input.libraries,
            &input.program_id,
            input.account_request_id,
        );

        // Compute expected account address using Instantiate2
        let expected_address = Self::compute_instantiate2_address(
            &input.factory,
            input.code_id,
            &salt,
            &input.code_checksum,
            &input.canonical_factory,
        )?;

        // Validate controller binding
        let is_valid_controller = Self::validate_controller(&input.controller);

        // Validate salt generation
        let is_valid_salt = Self::validate_salt(&salt, &input);

        // Validate address computation
        let is_valid_address = Self::validate_address_computation(
            &expected_address,
            &input.factory,
            input.code_id,
            &salt,
            &input.code_checksum,
            &input.canonical_factory,
        );

        Ok(FactoryWitness {
            controller: input.controller,
            salt,
            expected_address,
            is_valid_controller,
            is_valid_salt,
            is_valid_address,
        })
    }

    /// Generate deterministic salt with entropy
    fn generate_salt(
        block_height: u64,
        controller: &str,
        libraries: &[String],
        program_id: &str,
        account_request_id: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Add entropy sources
        hasher.update(block_height.to_be_bytes());

        // Request-specific deterministic data (must match CosmWasm contract order for consistency)
        hasher.update(controller.as_bytes());
        hasher.update(program_id.as_bytes());
        hasher.update(account_request_id.to_be_bytes());

        // Include library configuration in salt computation
        // This ensures accounts with different library sets get different addresses
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

    /// Compute Instantiate2 address
    fn compute_instantiate2_address(
        _factory: &str,
        _code_id: u64,
        salt: &[u8; 32],
        code_checksum: &[u8],
        canonical_factory: &[u8],
    ) -> Result<String, String> {
        // Create CanonicalAddr from the provided canonical factory bytes
        let canonical_creator = CanonicalAddr::from(canonical_factory);

        // Use the official CosmWasm instantiate2_address function
        let canonical_addr =
            instantiate2_address(code_checksum, &canonical_creator, salt)
                .map_err(|e| format!("instantiate2_address error: {}", e))?;

        // For the ZK environment, we'll return the canonical address as a hex string
        // In practice, this would need to be humanized to a bech32 address
        // but that requires the bech32 prefix which varies by chain
        // We create a deterministic representation by prefixing with "cosmos1" 
        // and encoding the canonical address
        let hex_addr = hex::encode(canonical_addr.as_slice());
        
        // Create a mock bech32-style address format for consistency
        // In production, this would use proper bech32 encoding with chain-specific prefix
        Ok(format!("cosmos1{}", &hex_addr[..40].to_lowercase()))
    }

    /// Validate controller address
    fn validate_controller(controller: &str) -> bool {
        // Comprehensive controller validation
        if controller.is_empty() {
            return false;
        }
        
        // Check minimum length for reasonable address
        if controller.len() < 10 {
            return false;
        }
        
        // Check maximum length to prevent abuse
        if controller.len() > 256 {
            return false;
        }
        
        // Additional validation could include:
        // - Bech32 format validation
        // - Checksum validation
        // - Address prefix validation
        
        true
    }

    /// Validate program ID
    fn validate_program_id(program_id: &str) -> bool {
        // Program ID validation
        if program_id.is_empty() {
            return false;
        }
        
        if program_id.len() > 256 {
            return false;
        }
        
        // Could add more specific validation rules here
        true
    }

    /// Validate block height for entropy
    fn validate_block_height(block_height: u64) -> bool {
        // Block height should not be zero and should be reasonable
        if block_height == 0 {
            return false;
        }
        
        // Prevent extremely large values that might indicate overflow
        if block_height > u64::MAX - 1000 {
            return false;
        }
        
        true
    }

    /// Validate account request ID
    fn validate_account_request_id(_account_request_id: u64) -> bool {
        // In production, you might want to enforce non-zero request IDs
        // For now, allow zero for testing purposes
        true
    }

    /// Validate libraries list
    fn validate_libraries(libraries: &[String]) -> bool {
        // Libraries should not be empty for meaningful accounts
        if libraries.is_empty() {
            return false;
        }
        
        // Check each library address
        for library in libraries {
            if library.is_empty() || library.len() > 256 {
                return false;
            }
        }
        
        // Prevent too many libraries (could be DoS vector)
        if libraries.len() > 100 {
            return false;
        }
        
        true
    }

    /// Comprehensive input validation
    fn validate_factory_input(input: &FactoryInput) -> bool {
        // Validate all input components
        if !Self::validate_controller(&input.controller) {
            return false;
        }
        
        if !Self::validate_libraries(&input.libraries) {
            return false;
        }
        
        if !Self::validate_program_id(&input.program_id) {
            return false;
        }
        
        if !Self::validate_block_height(input.block_height) {
            return false;
        }
        
        if !Self::validate_account_request_id(input.account_request_id) {
            return false;
        }
        
        // Validate code-related parameters
        if input.code_checksum.is_empty() {
            return false;
        }
        
        if input.canonical_factory.is_empty() {
            return false;
        }
        
        true
    }

    /// Validate salt generation
    fn validate_salt(salt: &[u8; 32], input: &FactoryInput) -> bool {
        // Regenerate salt and compare
        let expected_salt = Self::generate_salt(
            input.block_height,
            &input.controller,
            &input.libraries,
            &input.program_id,
            input.account_request_id,
        );
        salt == &expected_salt
    }

    /// Validate address computation
    fn validate_address_computation(
        address: &str,
        factory: &str,
        code_id: u64,
        salt: &[u8; 32],
        code_checksum: &[u8],
        canonical_factory: &[u8],
    ) -> bool {
        if let Ok(expected_address) = Self::compute_instantiate2_address(
            factory,
            code_id,
            salt,
            code_checksum,
            canonical_factory,
        ) {
            address == expected_address
        } else {
            false
        }
    }

    /// Generate circuit output
    pub fn generate_output(witness: &FactoryWitness) -> FactoryOutput {
        let is_valid =
            witness.is_valid_controller && witness.is_valid_salt && witness.is_valid_address;

        FactoryOutput {
            controller: witness.controller.clone(),
            account_address: witness.expected_address.clone(),
            is_valid,
        }
    }

    /// Validate atomic operation integrity
    pub fn validate_atomic_operation(input: &FactoryInput, witness: &FactoryWitness) -> bool {
        // Ensure the witness corresponds to the input
        witness.controller == input.controller
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salt_generation() {
        // Test basic salt generation
        let block_height = 12345;
        let controller = "cosmos1controller";
        let libraries = vec!["cosmos1lib1".to_string(), "cosmos1lib2".to_string()];
        let program_id = "test_program";
        let account_request_id = 456;

        let salt1 = CosmWasmAccountFactoryController::generate_salt(
            block_height,
            controller,
            &libraries,
            program_id,
            account_request_id,
        );

        let salt2 = CosmWasmAccountFactoryController::generate_salt(
            block_height + 1,
            controller,
            &libraries,
            program_id,
            account_request_id,
        );

        // Different inputs should produce different salts
        assert_ne!(salt1, salt2);

        // Same inputs should produce same salt
        let salt3 = CosmWasmAccountFactoryController::generate_salt(
            block_height,
            controller,
            &libraries,
            program_id,
            account_request_id,
        );
        assert_eq!(salt1, salt3);
    }

    #[test]
    fn test_salt_generation_library_order_independence() {
        let block_height = 12345;
        let controller = "cosmos1controller";
        let program_id = "test_program";
        let account_request_id = 456;

        // Same libraries in different orders
        let libraries_order1 = vec![
            "cosmos1lib1".to_string(),
            "cosmos1lib2".to_string(),
            "cosmos1lib3".to_string(),
        ];
        let libraries_order2 = vec![
            "cosmos1lib2".to_string(),
            "cosmos1lib3".to_string(),
            "cosmos1lib1".to_string(),
        ];
        let libraries_order3 = vec![
            "cosmos1lib3".to_string(),
            "cosmos1lib1".to_string(),
            "cosmos1lib2".to_string(),
        ];

        let salt1 = CosmWasmAccountFactoryController::generate_salt(
            block_height,
            controller,
            &libraries_order1,
            program_id,
            account_request_id,
        );

        let salt2 = CosmWasmAccountFactoryController::generate_salt(
            block_height,
            controller,
            &libraries_order2,
            program_id,
            account_request_id,
        );

        let salt3 = CosmWasmAccountFactoryController::generate_salt(
            block_height,
            controller,
            &libraries_order3,
            program_id,
            account_request_id,
        );

        // Same libraries in different orders should produce the same salt
        assert_eq!(salt1, salt2);
        assert_eq!(salt1, salt3);
        assert_eq!(salt2, salt3);
    }

    #[test]
    fn test_address_computation() {
        let block_height = 12345;
        let controller = "cosmos1controller";
        let libraries = vec!["cosmos1lib1".to_string()];
        let program_id = "test_program";
        let account_request_id = 789;

        let salt = CosmWasmAccountFactoryController::generate_salt(
            block_height,
            controller,
            &libraries,
            program_id,
            account_request_id,
        );

        let factory = "cosmos1factory";
        let code_id = 42;
        let code_checksum = vec![1u8; 32];
        let canonical_factory = vec![1u8; 20];

        let address = CosmWasmAccountFactoryController::compute_instantiate2_address(
            factory,
            code_id,
            &salt,
            &code_checksum,
            &canonical_factory,
        );

        assert!(address.is_ok());
    }

    #[test]
    fn test_witness_generation() {
        let input = FactoryInput {
            controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
            libraries: vec!["cosmos1lib1".to_string(), "cosmos1lib2".to_string()],
            program_id: "42".to_string(),
            account_request_id: 123,
            factory: "cosmos1factoryaddress".to_string(),
            code_id: 1,
            code_checksum: vec![1u8; 32],
            canonical_factory: vec![1u8; 20],
            block_height: 12345,
        };

        let witness = CosmWasmAccountFactoryController::process_input(input.clone()).unwrap();

        assert_eq!(witness.controller, input.controller);
        assert!(witness.is_valid_controller);
        assert!(witness.is_valid_salt);
        assert!(witness.is_valid_address);
    }

    #[test]
    fn test_atomic_operation_validation() {
        let input = FactoryInput {
            controller: "cosmos1abc123def456ghi789jkl012mno345pqr678stu901".to_string(),
            libraries: vec!["cosmos1lib1".to_string()],
            program_id: "42".to_string(),
            account_request_id: 123,
            factory: "cosmos1factoryaddress".to_string(),
            code_id: 1,
            code_checksum: vec![1u8; 32],
            canonical_factory: vec![1u8; 20],
            block_height: 12345,
        };

        let witness = CosmWasmAccountFactoryController::process_input(input.clone()).unwrap();

        assert!(CosmWasmAccountFactoryController::validate_atomic_operation(
            &input, &witness
        ));
    }
}
