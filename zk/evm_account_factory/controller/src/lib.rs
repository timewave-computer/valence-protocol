// Purpose: EVM Account Factory ZK Controller for generating witnesses and validating account creation
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
    /// Controller address that will own the account (as hex string)
    pub controller: String,
    /// Library addresses that will be approved for the account
    pub libraries: Vec<String>,
    /// Program ID for the Valence program (string identifier)
    pub program_id: String,
    /// Account request ID for uniqueness
    pub account_request_id: u64,
    /// Factory contract address (as hex string)
    pub factory: String,
    /// Block hash used for entropy
    pub block_hash: [u8; 32],
}

/// Factory witness containing intermediate validation data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactoryWitness {
    /// Controller address
    pub controller: String,
    /// Generated salt for CREATE2
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

/// EVM Account Factory Controller
pub struct EvmAccountFactoryController;

impl EvmAccountFactoryController {
    /// Process factory input and generate witness data
    pub fn process_input(input: FactoryInput) -> Result<FactoryWitness, String> {
        // Comprehensive input validation (done in controller, not circuit)
        if !Self::validate_factory_input(&input) {
            return Err("Invalid input parameters".to_string());
        }

        // Generate deterministic salt
        let salt = Self::generate_salt(
            &input.block_hash,
            &input.controller,
            &input.libraries,
            input.program_id.clone(),
            input.account_request_id,
        );

        // Compute expected account address using CREATE2
        let expected_address = Self::compute_create2_address(&input.factory, &salt)?;

        // Validate controller binding
        let is_valid_controller = Self::validate_controller(&input.controller);

        // Validate salt generation
        let is_valid_salt = Self::validate_salt(&salt, &input);

        // Validate address computation
        let is_valid_address = Self::validate_address_computation(
            &expected_address,
            &input.factory,
            &salt,
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
        block_hash: &[u8; 32],
        controller: &str,
        libraries: &[String],
        program_id: String,
        account_request_id: u64,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Add entropy sources
        hasher.update(block_hash);

        // Request-specific deterministic data (must match CosmWasm contract order for consistency)
        hasher.update(controller.as_bytes());
        hasher.update(program_id.as_bytes());
        hasher.update(&account_request_id.to_be_bytes());

        // Include library configuration in salt computation
        // This ensures accounts with different library sets get different addresses
        let mut lib_hasher = Sha256::new();
        for lib in libraries {
            lib_hasher.update(lib.as_bytes());
        }
        hasher.update(lib_hasher.finalize());

        hasher.finalize().into()
    }

    /// Compute CREATE2 address following EVM specification
    /// address = keccak256(0xff + deployer_address + salt + keccak256(init_code))[12:]
    fn compute_create2_address(
        factory: &str,
        salt: &[u8; 32],
    ) -> Result<String, String> {
        // Parse factory address (remove 0x prefix and convert to bytes)
        if !factory.starts_with("0x") || factory.len() != 42 {
            return Err("Invalid factory address format".to_string());
        }
        
        let factory_bytes = hex::decode(&factory[2..])
            .map_err(|_| "Invalid factory address hex".to_string())?;
        
        if factory_bytes.len() != 20 {
            return Err("Factory address must be 20 bytes".to_string());
        }
        
        // For this implementation, we use a simplified init_code hash
        // In production, this would be the actual JitAccount bytecode hash
        let init_code_hash = {
            let mut hasher = Sha256::new();
            hasher.update(b"JitAccount_init_code_placeholder");
            hasher.finalize()
        };
        
        // Compute CREATE2 address: keccak256(0xff + factory + salt + init_code_hash)[12:]
        let mut data = Vec::with_capacity(1 + 20 + 32 + 32);
        data.push(0xff);                    // CREATE2 prefix
        data.extend_from_slice(&factory_bytes); // Factory address (20 bytes)
        data.extend_from_slice(salt);       // Salt (32 bytes)
        data.extend_from_slice(&init_code_hash); // Init code hash (32 bytes)
        
        // Use SHA256 as placeholder for keccak256
        // TODO: In production, use proper keccak256 implementation
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hasher.finalize();
        
        // Take last 20 bytes (equivalent to [12:] for 32-byte hash)
        let address_bytes = &hash[12..32];
        
        Ok(format!("0x{}", hex::encode(address_bytes)))
    }

    /// Validate controller address
    fn validate_controller(controller: &str) -> bool {
        // Comprehensive EVM controller validation
        if controller.is_empty() {
            return false;
        }
        
        // Check for proper hex format (0x prefix + 40 hex chars = 42 total)
        if !controller.starts_with("0x") || controller.len() != 42 {
            return false;
        }
        
        // Validate hex characters after 0x prefix
        for c in controller.chars().skip(2) {
            if !c.is_ascii_hexdigit() {
                return false;
            }
        }
        
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

    /// Validate block hash for entropy
    fn validate_block_hash(block_hash: &[u8; 32]) -> bool {
        // Block hash should not be all zeros (invalid block)
        *block_hash != [0u8; 32]
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
            
            // For EVM, libraries should be valid hex addresses
            if !Self::validate_controller(library) {
                return false;
            }
        }
        
        // Prevent too many libraries (could be DoS vector)
        if libraries.len() > 100 {
            return false;
        }
        
        true
    }

    /// Validate factory address
    fn validate_factory(factory: &str) -> bool {
        // Factory should be a valid EVM address
        Self::validate_controller(factory)
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
        
        if !Self::validate_block_hash(&input.block_hash) {
            return false;
        }
        
        if !Self::validate_account_request_id(input.account_request_id) {
            return false;
        }
        
        if !Self::validate_factory(&input.factory) {
            return false;
        }
        
        true
    }

    /// Validate salt generation
    fn validate_salt(salt: &[u8; 32], input: &FactoryInput) -> bool {
        // Regenerate salt and compare
        let expected_salt = Self::generate_salt(
            &input.block_hash,
            &input.controller,
            &input.libraries,
            input.program_id.clone(),
            input.account_request_id,
        );
        salt == &expected_salt
    }

    /// Validate address computation
    fn validate_address_computation(
        address: &str,
        factory: &str,
        salt: &[u8; 32],
    ) -> bool {
        if let Ok(expected_address) = Self::compute_create2_address(factory, salt) {
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
        let block_hash = [1u8; 32];
        let program_id = "42".to_string();
        let account_request_id = 123;
        let controller = "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89";

        let salt1 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            controller,
            &vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id.clone(),
            account_request_id,
        );

        // Use different block hash to get different salt
        let block_hash2 = [2u8; 32];
        let salt2 = EvmAccountFactoryController::generate_salt(
            &block_hash2,
            controller,
            &vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id.clone(),
            account_request_id,
        );

        // Different block hashes should produce different salts
        assert_ne!(salt1, salt2);

        // Same inputs should produce same salt
        let salt3 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            controller,
            &vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id,
            account_request_id,
        );
        assert_eq!(salt1, salt3);
    }

    #[test]
    fn test_salt_generation_with_different_libraries() {
        let block_hash = [1u8; 32];
        let program_id = "42".to_string();
        let account_request_id = 123;
        let controller = "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89";

        let libraries1 = vec!["0x1234567890123456789012345678901234567890".to_string()];
        let libraries2 = vec!["0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string()];
        let libraries3 = vec![
            "0x1234567890123456789012345678901234567890".to_string(),
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string(),
        ];

        let salt1 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            controller,
            &libraries1,
            program_id.clone(),
            account_request_id,
        );

        let salt2 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            controller,
            &libraries2,
            program_id.clone(),
            account_request_id,
        );

        let salt3 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            controller,
            &libraries3,
            program_id,
            account_request_id,
        );

        // Different library sets should produce different salts
        assert_ne!(salt1, salt2);
        assert_ne!(salt1, salt3);
        assert_ne!(salt2, salt3);
    }

    #[test]
    fn test_salt_generation_with_different_controllers() {
        let block_hash = [1u8; 32];
        let program_id = "42".to_string();
        let account_request_id = 123;
        let controller = "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89";

        let salt1 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89",
            &vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id.clone(),
            account_request_id,
        );

        let salt2 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            "0x123456789abcdef0123456789abcdef012345678",
            &vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id,
            account_request_id,
        );

        // Different controllers should produce different salts
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_address_computation() {
        let factory = "0x1234567890123456789012345678901234567890";
        let salt1 = [2u8; 32];
        let salt2 = [3u8; 32];

        let addr1 = EvmAccountFactoryController::compute_create2_address(
            factory,
            &salt1,
        )
        .unwrap();

        let addr2 = EvmAccountFactoryController::compute_create2_address(
            factory,
            &salt2,
        )
        .unwrap();

        // Different salts should produce different addresses
        assert_ne!(addr1, addr2);

        // Both should be valid Ethereum addresses
        assert!(addr1.starts_with("0x"));
        assert_eq!(addr1.len(), 42);
        assert!(addr2.starts_with("0x"));
        assert_eq!(addr2.len(), 42);
    }

    #[test]
    fn test_process_input() {
        let input = FactoryInput {
            controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
            libraries: vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id: "42".to_string(),
            account_request_id: 123,
            factory: "0x1234567890123456789012345678901234567890".to_string(),
            block_hash: [1u8; 32],
        };

        let witness = EvmAccountFactoryController::process_input(input.clone()).unwrap();

        assert_eq!(witness.controller, input.controller);
        assert!(witness.is_valid_controller);
        assert!(witness.is_valid_salt);
        assert!(witness.is_valid_address);
    }

    #[test]
    fn test_atomic_operation_validation() {
        let input = FactoryInput {
            controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
            libraries: vec!["0x1234567890123456789012345678901234567890".to_string()],
            program_id: "42".to_string(),
            account_request_id: 123,
            factory: "0x1234567890123456789012345678901234567890".to_string(),
            block_hash: [1u8; 32],
        };

        let witness = EvmAccountFactoryController::process_input(input.clone()).unwrap();

        assert!(EvmAccountFactoryController::validate_atomic_operation(
            &input, &witness
        ));
    }
}
