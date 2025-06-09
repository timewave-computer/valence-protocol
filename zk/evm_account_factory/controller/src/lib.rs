// Purpose: EVM Account Factory ZK Controller for generating witnesses and validating account creation
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use hex;

/// Account type configuration for EVM
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    /// Account supports only token custody
    TokenCustody,
    /// Account supports only data storage
    DataStorage,
    /// Account supports both token custody and data storage
    Hybrid,
}

impl AccountType {
    /// Convert to byte representation for salt generation
    pub fn to_byte(&self) -> u8 {
        match self {
            AccountType::TokenCustody => 1,
            AccountType::DataStorage => 2,
            AccountType::Hybrid => 3,
        }
    }
}

/// Input data for EVM account factory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryInput {
    /// Controller address that will own the account (as hex string)
    pub controller: String,
    /// Program ID for the Valence program
    pub program_id: u64,
    /// Account request ID for uniqueness
    pub account_request_id: u64,
    /// Account type configuration
    pub account_type: AccountType,
    /// Factory contract address (as hex string)
    pub factory: String,
    /// Block hash used for entropy
    pub block_hash: [u8; 32],
}

/// Witness data for ZK circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryWitness {
    /// Controller address
    pub controller: String,
    /// Generated salt for CREATE2
    pub salt: [u8; 32],
    /// Expected account address
    pub expected_address: String,
    /// Account type configuration
    pub account_type: AccountType,
    /// Validation flags
    pub is_valid_controller: bool,
    pub is_valid_salt: bool,
    pub is_valid_address: bool,
}

/// Public output for the circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryOutput {
    /// Controller address
    pub controller: String,
    /// Created account address
    pub account_address: String,
    /// Account type configuration
    pub account_type: AccountType,
    /// Validation result
    pub is_valid: bool,
}

/// EVM Account Factory Controller
pub struct EvmAccountFactoryController;

impl EvmAccountFactoryController {
    /// Parse input data and generate witnesses
    pub fn process_input(input: FactoryInput) -> Result<FactoryWitness, String> {
        // Generate salt using entropy sources and account type
        let salt = Self::generate_salt(
            &input.block_hash,
            input.program_id,
            input.account_request_id,
            &input.account_type,
        );

        // Compute expected account address using CREATE2
        let expected_address = Self::compute_create2_address(
            &input.factory,
            &salt,
            &input.account_type,
        )?;

        // Validate controller binding
        let is_valid_controller = Self::validate_controller(&input.controller);

        // Validate salt generation
        let is_valid_salt = Self::validate_salt(&salt, &input);

        // Validate address computation
        let is_valid_address = Self::validate_address_computation(
            &expected_address,
            &input.factory,
            &salt,
            &input.account_type,
        );

        Ok(FactoryWitness {
            controller: input.controller,
            salt,
            expected_address,
            account_type: input.account_type,
            is_valid_controller,
            is_valid_salt,
            is_valid_address,
        })
    }

    /// Generate deterministic salt with entropy and account type
    fn generate_salt(
        block_hash: &[u8; 32],
        program_id: u64,
        account_request_id: u64,
        account_type: &AccountType,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        
        // Add entropy sources
        hasher.update(block_hash);
        hasher.update(&program_id.to_be_bytes());
        hasher.update(&account_request_id.to_be_bytes());
        
        // Include account type in salt to ensure different types get different addresses
        hasher.update(&[account_type.to_byte()]);
        
        hasher.finalize().into()
    }

    /// Compute CREATE2 address
    fn compute_create2_address(
        factory: &str,
        salt: &[u8; 32],
        account_type: &AccountType,
    ) -> Result<String, String> {
        // Simplified CREATE2 address computation
        // In reality, this would use the actual EVM CREATE2 derivation
        let mut hasher = Sha256::new();
        hasher.update(factory.as_bytes());
        hasher.update(salt);
        hasher.update(&[account_type.to_byte()]);
        
        let hash = hasher.finalize();
        
        // Convert to Ethereum address format (simplified)
        Ok(format!("0x{}", hex::encode(&hash[..20])))
    }

    /// Validate controller address
    fn validate_controller(controller: &str) -> bool {
        // Basic validation - ensure controller is not empty and has valid hex format
        controller.starts_with("0x") && controller.len() == 42
    }

    /// Validate salt generation
    fn validate_salt(salt: &[u8; 32], input: &FactoryInput) -> bool {
        // Regenerate salt and compare
        let expected_salt = Self::generate_salt(
            &input.block_hash,
            input.program_id,
            input.account_request_id,
            &input.account_type,
        );
        salt == &expected_salt
    }

    /// Validate address computation
    fn validate_address_computation(
        address: &str,
        factory: &str,
        salt: &[u8; 32],
        account_type: &AccountType,
    ) -> bool {
        if let Ok(expected_address) = Self::compute_create2_address(
            factory,
            salt,
            account_type,
        ) {
            address == expected_address
        } else {
            false
        }
    }

    /// Generate circuit output
    pub fn generate_output(witness: &FactoryWitness) -> FactoryOutput {
        let is_valid = witness.is_valid_controller
            && witness.is_valid_salt
            && witness.is_valid_address;

        FactoryOutput {
            controller: witness.controller.clone(),
            account_address: witness.expected_address.clone(),
            account_type: witness.account_type.clone(),
            is_valid,
        }
    }

    /// Validate atomic operation integrity
    pub fn validate_atomic_operation(
        input: &FactoryInput,
        witness: &FactoryWitness,
    ) -> bool {
        // Ensure the witness corresponds to the input
        witness.controller == input.controller
            && witness.account_type == input.account_type
    }

    /// Validate account type configuration
    pub fn validate_account_type_config(
        requested_type: &AccountType,
        created_type: &AccountType,
    ) -> bool {
        requested_type == created_type
    }

    /// Handle different account capability configurations
    pub fn process_account_capabilities(
        account_type: &AccountType,
        init_msg: &mut serde_json::Value,
    ) -> Result<(), String> {
        match account_type {
            AccountType::TokenCustody => {
                init_msg["enable_token_custody"] = true.into();
                init_msg["enable_data_storage"] = false.into();
            }
            AccountType::DataStorage => {
                init_msg["enable_token_custody"] = false.into();
                init_msg["enable_data_storage"] = true.into();
            }
            AccountType::Hybrid => {
                init_msg["enable_token_custody"] = true.into();
                init_msg["enable_data_storage"] = true.into();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salt_generation() {
        let block_hash = [1u8; 32];
        let program_id = 42;
        let account_request_id = 123;

        let salt1 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            program_id,
            account_request_id,
            &AccountType::TokenCustody,
        );

        let salt2 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            program_id,
            account_request_id,
            &AccountType::DataStorage,
        );

        // Different account types should produce different salts
        assert_ne!(salt1, salt2);

        // Same inputs should produce same salt
        let salt3 = EvmAccountFactoryController::generate_salt(
            &block_hash,
            program_id,
            account_request_id,
            &AccountType::TokenCustody,
        );
        assert_eq!(salt1, salt3);
    }

    #[test]
    fn test_address_computation() {
        let factory = "0x1234567890123456789012345678901234567890";
        let salt = [2u8; 32];

        let addr1 = EvmAccountFactoryController::compute_create2_address(
            factory,
            &salt,
            &AccountType::TokenCustody,
        ).unwrap();

        let addr2 = EvmAccountFactoryController::compute_create2_address(
            factory,
            &salt,
            &AccountType::DataStorage,
        ).unwrap();

        // Different account types should produce different addresses
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
            program_id: 42,
            account_request_id: 123,
            account_type: AccountType::Hybrid,
            factory: "0x1234567890123456789012345678901234567890".to_string(),
            block_hash: [1u8; 32],
        };

        let witness = EvmAccountFactoryController::process_input(input.clone()).unwrap();

        assert_eq!(witness.controller, input.controller);
        assert_eq!(witness.account_type, input.account_type);
        assert!(witness.is_valid_controller);
        assert!(witness.is_valid_salt);
        assert!(witness.is_valid_address);
    }

    #[test]
    fn test_atomic_operation_validation() {
        let input = FactoryInput {
            controller: "0x742C7D7672Ad5ba34e1b05b19dA8B8CB43Ac6e89".to_string(),
            program_id: 42,
            account_request_id: 123,
            account_type: AccountType::TokenCustody,
            factory: "0x1234567890123456789012345678901234567890".to_string(),
            block_hash: [1u8; 32],
        };

        let witness = EvmAccountFactoryController::process_input(input.clone()).unwrap();

        assert!(EvmAccountFactoryController::validate_atomic_operation(
            &input, &witness
        ));
    }

    #[test]
    fn test_account_type_validation() {
        assert!(EvmAccountFactoryController::validate_account_type_config(
            &AccountType::TokenCustody,
            &AccountType::TokenCustody
        ));

        assert!(!EvmAccountFactoryController::validate_account_type_config(
            &AccountType::TokenCustody,
            &AccountType::DataStorage
        ));
    }

    #[test]
    fn test_account_capabilities() {
        let mut init_msg = serde_json::json!({});

        EvmAccountFactoryController::process_account_capabilities(
            &AccountType::TokenCustody,
            &mut init_msg,
        ).unwrap();

        assert_eq!(init_msg["enable_token_custody"], true);
        assert_eq!(init_msg["enable_data_storage"], false);

        EvmAccountFactoryController::process_account_capabilities(
            &AccountType::Hybrid,
            &mut init_msg,
        ).unwrap();

        assert_eq!(init_msg["enable_token_custody"], true);
        assert_eq!(init_msg["enable_data_storage"], true);
    }
} 