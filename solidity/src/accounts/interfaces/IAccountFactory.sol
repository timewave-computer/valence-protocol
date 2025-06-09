// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.28;

/**
 * @title IAccountFactory
 * @dev Interface for deterministic account creation with atomic request processing
 * Enables entities to create accounts with predictable addresses using CREATE2
 */
interface IAccountFactory {
    /// @notice Data structure for account creation requests
    struct AccountRequest {
        address controller;      // Address that will control the account
        address[] libraries;     // Initial approved libraries
        bytes32 programId;       // Unique program identifier
        uint256 nonce;           // Unique nonce for this request
        bytes signature;         // Authorization signature
    }

    /// @notice Data structure for batch operations
    struct BatchRequest {
        AccountRequest[] requests;
        address ferry;          // Ferry service operator address
        uint256 fee;            // Fee for ferry service
    }

    /// @notice Emitted when an account is created
    event AccountCreated(
        address indexed account,
        address indexed controller,
        bytes32 indexed programId,
        bytes32 salt
    );

    /// @notice Emitted when a batch operation is processed
    event BatchProcessed(
        address indexed ferry,
        uint256 requestCount,
        uint256 totalFee
    );

    /**
     * @dev Creates a new account with deterministic address
     * @param request Account creation request data
     * @return account Address of the created account
     */
    function createAccount(AccountRequest calldata request) external returns (address account);

    /**
     * @dev Creates account and processes request atomically
     * @param request Account creation request data
     * @return account Address of the created account
     */
    function createAccountWithRequest(AccountRequest calldata request) external returns (address account);

    /**
     * @dev Processes multiple account creations in batch
     * @param batch Batch request data
     * @return accounts Array of created account addresses
     */
    function createAccountsBatch(BatchRequest calldata batch) external returns (address[] memory accounts);

    /**
     * @dev Computes the deterministic address for an account
     * @param request Account creation request data
     * @return account Predicted address of the account
     */
    function computeAccountAddress(AccountRequest calldata request) external view returns (address account);

    /**
     * @dev Checks if an account has been created for given parameters
     * @param account Address to check
     * @return created True if account exists
     */
    function isAccountCreated(address account) external view returns (bool created);
} 