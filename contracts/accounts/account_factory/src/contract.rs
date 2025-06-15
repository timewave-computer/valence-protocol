// Purpose: Account factory contract with Instantiate2-based deterministic creation
//
// This contract provides deterministic account creation using CosmWasm's Instantiate2
// message type. It supports atomic operations where request validation and account
// creation happen in a single transaction, and includes ferry service support for
// batch processing by off-chain operators.
//
// Key Features:
// - Deterministic addressing via salt-based Instantiate2
// - Atomic account creation with request validation
// - Nonce-based replay protection
// - Full capability accounts (both token custody and data storage)
// - Ferry service batch processing with fee collection

use cosmwasm_crypto::secp256k1_verify;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, 
    Env, MessageInfo, Response, StdError, StdResult, WasmMsg,
};
use cw2::set_contract_version;
use sha2::{Digest, Sha256};

use crate::msg::{
    AccountRequest, AccountRequestForSigning, BatchRequest, ComputeAccountAddressResponse,
    ExecuteMsg, InstantiateMsg, QueryMsg, MAX_BLOCK_AGE,
};
use crate::state::{CREATED_ACCOUNTS, FEE_COLLECTOR, JIT_ACCOUNT_CODE_ID, USED_NONCES};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Contract-specific errors
#[derive(thiserror::Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Account already exists: {account}")]
    AccountAlreadyExists { account: Addr },
    #[error("Invalid controller address")]
    InvalidController {},
    #[error("Account request ID already used: {controller} - {account_request_id}")]
    AccountRequestIdAlreadyUsed {
        controller: Addr,
        account_request_id: u64,
    },
    #[error("Historical block too old: current {current_height}, historical {historical_height}")]
    HistoricalBlockTooOld {
        current_height: u64,
        historical_height: u64,
    },
    #[error("Invalid signature")]
    InvalidSignature {},
    #[error("Insufficient fee")]
    InsufficientFee {},
}

/// Initialize the account factory contract
///
/// Sets up the factory with a JIT account code ID for Instantiate2 and optionally
/// configures a fee collector address for ferry service operations.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Store the code ID for JIT accounts that will be created
    JIT_ACCOUNT_CODE_ID.save(deps.storage, &msg.jit_account_code_id)?;

    // Configure optional fee collector for ferry services
    let fee_collector = match msg.fee_collector {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };
    FEE_COLLECTOR.save(deps.storage, &fee_collector)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("jit_account_code_id", msg.jit_account_code_id.to_string()))
}

/// Main entry point for executing factory operations
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Standard account creation - validate and create account
        ExecuteMsg::CreateAccount { request } => execute::create_account(deps, env, request),

        // Atomic account creation - validate request signature and create account atomically
        ExecuteMsg::CreateAccountWithRequest { request } => {
            execute::create_account_with_request(deps, env, request)
        }

        // Batch processing for ferry services - create multiple accounts efficiently
        ExecuteMsg::CreateAccountsBatch { batch } => {
            execute::create_accounts_batch(deps, env, info, batch)
        }
    }
}

/// Internal execute module containing the implementation logic
pub mod execute {
    use super::*;

    /// Create a single account with basic validation
    ///
    /// Validates the request for nonce uniqueness and controller validity,
    /// then creates the account using Instantiate2 for deterministic addressing.
    pub fn create_account(
        deps: DepsMut,
        env: Env,
        request: AccountRequest,
    ) -> Result<Response, ContractError> {
        // Validate request parameters
        validate_request(&deps, &env, &request)?;

        // Create the account and update state
        let (account_addr, msgs) = create_account_internal(deps, env, request)?;

        Ok(Response::new()
            .add_messages(msgs)
            .add_attribute("method", "create_account")
            .add_attribute("account", account_addr))
    }

    /// Create an account atomically with request validation
    ///
    /// This method validates both the request structure and any provided signature,
    /// then creates the account in a single atomic operation. If any step fails,
    /// the entire operation is reverted.
    pub fn create_account_with_request(
        deps: DepsMut,
        env: Env,
        request: AccountRequest,
    ) -> Result<Response, ContractError> {
        // Validate basic request structure
        validate_request(&deps, &env, &request)?;

        // For atomic operations, validate signature if provided
        if request.signature.is_some() {
            validate_signature(&deps, &request)?;
        }

        // Create account atomically
        let (account_addr, msgs) = create_account_internal(deps, env, request)?;

        Ok(Response::new()
            .add_messages(msgs)
            .add_attribute("method", "create_account_with_request")
            .add_attribute("account", account_addr))
    }

    /// Process multiple account creation requests in a batch
    ///
    /// This method is optimized for ferry services and off-chain operators
    /// who need to process multiple account creations efficiently. It includes
    /// fee collection mechanisms and batch optimization.
    pub fn create_accounts_batch(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        batch: BatchRequest,
    ) -> Result<Response, ContractError> {
        // Validate batch is not empty
        if batch.requests.is_empty() {
            return Err(ContractError::Std(StdError::generic_err(
                "Batch cannot be empty",
            )));
        }

        let mut accounts = Vec::new();
        let mut messages = Vec::new();

        // Process each request in the batch
        for request in batch.requests {
            // Validate each request
            validate_request(&deps, &env, &request)?;

            // Prepare account creation without actually executing
            let (account_addr, msgs) = prepare_account_creation(&deps.as_ref(), &env, &request)?;
            accounts.push(account_addr.clone());
            messages.extend(msgs);

            // Update state for nonce and account tracking
            let controller = deps.api.addr_validate(&request.controller)?;
            USED_NONCES.save(
                deps.storage,
                (controller, request.account_request_id),
                &true,
            )?;
            CREATED_ACCOUNTS.save(deps.storage, account_addr, &true)?;
        }

        // Handle ferry service fee collection
        if batch.fee_amount > 0 {
            let fee_collector = match FEE_COLLECTOR.load(deps.storage)? {
                Some(addr) => addr,
                None => {
                    return Err(ContractError::Std(
                        StdError::generic_err("Fee collector not configured"),
                    ))
                }
            };

            // Require the caller to pay the exact fee in a single coin
            let paid = info
                .funds
                .iter()
                .find(|c| c.amount.u128() == batch.fee_amount)
                .ok_or(ContractError::InsufficientFee {})?;

            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: fee_collector.to_string(),
                amount: vec![paid.clone()],
            }));
        }

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("method", "create_accounts_batch")
            .add_attribute("ferry", batch.ferry)
            .add_attribute("account_count", accounts.len().to_string()))
    }

    /// Validate an account creation request
    ///
    /// Checks controller address validity, historical block age, and ensures account request ID hasn't been used
    /// to prevent replay attacks and ensure account uniqueness.
    fn validate_request(
        deps: &DepsMut,
        env: &Env,
        request: &AccountRequest,
    ) -> Result<(), ContractError> {
        // Validate controller address format
        let controller = deps
            .api
            .addr_validate(&request.controller)
            .map_err(|_| ContractError::InvalidController {})?;

        // Validate libraries list (should not be empty for meaningful accounts)
        if request.libraries.is_empty() {
            return Err(ContractError::Std(StdError::generic_err(
                "Libraries list cannot be empty. Accounts must have at least one approved library.",
            )));
        }

        // Validate all library addresses are valid
        let mut seen_libraries = std::collections::HashSet::new();
        for library in &request.libraries {
            deps.api.addr_validate(library).map_err(|_| {
                ContractError::Std(StdError::generic_err(format!(
                    "Invalid library address: {}",
                    library
                )))
            })?;

            // Check for duplicates
            if !seen_libraries.insert(library.clone()) {
                return Err(ContractError::Std(StdError::generic_err(format!(
                    "Duplicate library address: {}",
                    library
                ))));
            }
        }

        // Historical block must be strictly in the past and within MAX_BLOCK_AGE
        if request.historical_block_height >= env.block.height {
            return Err(ContractError::Std(StdError::generic_err(
                "Historical block height must be in the past",
            )));
        }
        // Check historical block age is within acceptable range
        if env.block.height > request.historical_block_height + MAX_BLOCK_AGE {
            return Err(ContractError::HistoricalBlockTooOld {
                current_height: env.block.height,
                historical_height: request.historical_block_height,
            });
        }

        // Check account request ID hasn't been used (prevents replay attacks)
        if USED_NONCES.has(
            deps.storage,
            (controller.clone(), request.account_request_id),
        ) {
            return Err(ContractError::AccountRequestIdAlreadyUsed {
                controller,
                account_request_id: request.account_request_id,
            });
        }

        Ok(())
    }

    /// Validate request signature for atomic operations
    ///
    /// This implementation uses secp256k1 signature verification to authenticate
    /// the request origin against the controller's public key. The signature is
    /// verified against the serialized request data to ensure authenticity.
    fn validate_signature(deps: &DepsMut, request: &AccountRequest) -> Result<(), ContractError> {
        match (&request.signature, &request.public_key) {
            (Some(signature_bytes), Some(public_key_bytes)) => {
                // Validate signature length (64 bytes for r,s)
                if signature_bytes.len() != 64 {
                    return Err(ContractError::InvalidSignature {});
                }

                // Validate public key length (33 bytes for compressed secp256k1)
                if public_key_bytes.len() != 33 {
                    return Err(ContractError::InvalidSignature {});
                }

                // Create message data for verification (excluding signature and public key fields)
                let request_for_signing = AccountRequestForSigning::from(request);
                let message_bytes = cosmwasm_std::to_json_vec(&request_for_signing)
                    .map_err(|_| ContractError::InvalidSignature {})?;

                // Hash the message using SHA-256 (standard for Cosmos ecosystem)
                let mut hasher = Sha256::new();
                hasher.update(&message_bytes);
                let message_hash = hasher.finalize();

                // Verify the signature against the message hash and public key
                match secp256k1_verify(&message_hash, signature_bytes, public_key_bytes) {
                    Ok(true) => {
                        // Derive address from public key and verify it matches controller
                        let derived_addr = derive_address_from_pubkey(&deps.as_ref(), public_key_bytes)?;
                        let controller_addr = deps.api.addr_validate(&request.controller)?;
                        
                        if derived_addr != controller_addr {
                            return Err(ContractError::Std(StdError::generic_err(
                                "Public key does not match controller address"
                            )));
                        }
                        
                        Ok(())
                    }
                    Ok(false) => Err(ContractError::InvalidSignature {}),
                    Err(_) => Err(ContractError::InvalidSignature {}),
                }
            }
            (Some(_), None) => {
                // Signature provided but no public key
                Err(ContractError::Std(StdError::generic_err(
                    "Public key required when signature is provided",
                )))
            }
            (None, Some(_)) => {
                // Public key provided but no signature
                Err(ContractError::Std(StdError::generic_err(
                    "Signature required when public key is provided",
                )))
            }
            (None, None) => {
                // Neither signature nor public key provided - this is valid for non-atomic operations
                Ok(())
            }
        }
    }

    /// Internal account creation logic
    ///
    /// Handles the core account creation process including salt generation,
    /// address computation, state updates, and message preparation.
    fn create_account_internal(
        deps: DepsMut,
        env: Env,
        request: AccountRequest,
    ) -> Result<(Addr, Vec<CosmosMsg>), ContractError> {
        let controller = deps.api.addr_validate(&request.controller)?;

        // Prepare account creation messages and compute address
        let (account_addr, msgs) = prepare_account_creation(&deps.as_ref(), &env, &request)?;

        // Update state to mark nonce as used and account as created
        USED_NONCES.save(
            deps.storage,
            (controller, request.account_request_id),
            &true,
        )?;
        CREATED_ACCOUNTS.save(deps.storage, account_addr.clone(), &true)?;

        Ok((account_addr, msgs))
    }

    /// Prepare account creation messages and compute address
    ///
    /// This function handles the deterministic address computation using
    /// Instantiate2 and prepares the instantiation message for the JIT account.
    fn prepare_account_creation(
        deps: &Deps,
        env: &Env,
        request: &AccountRequest,
    ) -> Result<(Addr, Vec<CosmosMsg>), ContractError> {
        // Generate deterministic salt from request parameters
        let salt = compute_salt(env, request);
        let code_id = JIT_ACCOUNT_CODE_ID.load(deps.storage)?;

        // Compute what the account address will be
        let account_addr =
            compute_instantiate2_address(deps, &env.contract.address, code_id, &salt)?;

        // Check if account already exists
        if CREATED_ACCOUNTS.has(deps.storage, account_addr.clone()) {
            return Err(ContractError::AccountAlreadyExists {
                account: account_addr,
            });
        }

        // Prepare JIT account instantiation message
        let instantiate_msg = crate::msg::JitAccountInstantiateMsg {
            controller: request.controller.clone(),
        };

        // Create the Wasm instantiation message
        let msg = CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: None,
            code_id,
            msg: to_json_binary(&instantiate_msg)?,
            funds: vec![],
            label: format!("valence-account-{}", request.program_id),
            salt: Binary::from(salt.to_vec()),
        });

        Ok((account_addr, vec![msg]))
    }

    /// Generate deterministic salt for account creation
    ///
    /// Combines multiple entropy sources including historical block data to create a unique,
    /// deterministic salt that prevents address collisions while allowing predictable address computation.
    ///
    /// Salt includes:
    /// - Historical block height (for temporal entropy)
    /// - Controller address (ensures isolation between controllers)
    /// - Program ID (allows multiple programs per controller)
    /// - Account request ID (user-provided uniqueness guarantee)
    /// - Libraries hash (accounts with different library sets get different addresses)
    pub fn compute_salt(_env: &Env, request: &AccountRequest) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Historical block-based entropy for temporal variation
        hasher.update(request.historical_block_height.to_le_bytes());

        // Request-specific deterministic data
        hasher.update(request.controller.as_bytes());
        hasher.update(request.program_id.as_bytes());
        hasher.update(request.account_request_id.to_le_bytes());

        // Include library configuration in salt computation
        // This ensures accounts with different library approvals get different addresses
        // Sort libraries to ensure deterministic ordering
        let mut libs = request.libraries.clone();
        libs.sort();
        let mut lib_hasher = Sha256::new();
        for lib in &libs {
            lib_hasher.update(lib.as_bytes());
        }
        hasher.update(lib_hasher.finalize());

        hasher.finalize().into()
    }

    /// Compute the deterministic address for an Instantiate2 operation
    ///
    /// This follows the CosmWasm Instantiate2 addressing algorithm to predict
    /// what address an account will have before actually creating it.
    #[cfg(not(test))]
    pub fn compute_instantiate2_address(
        deps: &Deps,
        factory_addr: &Addr,
        code_id: u64,
        salt: &[u8; 32],
    ) -> Result<Addr, ContractError> {
        // For the ZK controller to work properly, we need to get the code checksum
        // Query the code info to get the checksum
        let code_info = deps
            .querier
            .query_wasm_code_info(code_id)
            .map_err(ContractError::Std)?;

        // Use the official CosmWasm Instantiate2 derivation
        // 1. Turn the human address into its canonical form
        let canonical_creator = deps
            .api
            .addr_canonicalize(factory_addr.as_str())
            .map_err(ContractError::Std)?;

        // 2. Compute the raw (canonical) address using the built-in function
        let raw = instantiate2_address(code_info.checksum.as_slice(), &canonical_creator, salt)
            .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;

        // 3. Convert back to human-readable Bech32
        let human = deps.api.addr_humanize(&raw).map_err(ContractError::Std)?;

        Ok(human)
    }

    /// Test-only version of compute_instantiate2_address that uses a fixed checksum
    /// to avoid requiring code to be deployed in the mock environment
    #[cfg(test)]
    pub fn compute_instantiate2_address(
        deps: &Deps,
        factory_addr: &Addr,
        _code_id: u64,
        salt: &[u8; 32],
    ) -> Result<Addr, ContractError> {
        // Use a fixed checksum for testing (32 bytes of 0x01)
        let test_checksum = vec![0x01u8; 32];

        // Use the official CosmWasm Instantiate2 derivation
        // 1. Turn the human address into its canonical form
        let canonical_creator = deps
            .api
            .addr_canonicalize(factory_addr.as_str())
            .map_err(ContractError::Std)?;

        // 2. Compute the raw (canonical) address using the built-in function
        let raw = instantiate2_address(&test_checksum, &canonical_creator, salt)
            .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;

        // 3. Convert back to human-readable Bech32
        let human = deps.api.addr_humanize(&raw).map_err(ContractError::Std)?;

        Ok(human)
    }

    /// Derive a Cosmos address from a secp256k1 public key
    /// 
    /// This follows the standard Cosmos SDK address derivation:
    /// 1. Take the 33-byte compressed secp256k1 public key
    /// 2. Hash it with SHA256 
    /// 3. Take the first 20 bytes (RIPEMD160 in Cosmos SDK, but SHA256 in CosmWasm)
    /// 4. Convert to proper Bech32 address format using the chain's address derivation
    pub fn derive_address_from_pubkey(deps: &Deps, public_key: &[u8]) -> Result<Addr, ContractError> {
        // Ensure we have a valid compressed secp256k1 public key (33 bytes)
        if public_key.len() != 33 {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid public key length - expected 33 bytes for compressed secp256k1"
            )));
        }

        // Verify the public key format (should start with 0x02 or 0x03 for compressed keys)
        if public_key[0] != 0x02 && public_key[0] != 0x03 {
            return Err(ContractError::Std(StdError::generic_err(
                "Invalid compressed secp256k1 public key format"
            )));
        }

        // Hash the public key with SHA256
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();

        // Take the first 20 bytes for the address (this matches Cosmos SDK convention)
        let address_bytes = &hash[0..20];

        // Convert to canonical address format
        let canonical_addr = CanonicalAddr::from(address_bytes);
        
        // Use the proper CosmWasm API to convert to human-readable Bech32 address
        // This ensures the address matches on-chain expectations and uses the correct prefix
        let human_addr = deps.api.addr_humanize(&canonical_addr).map_err(ContractError::Std)?;

        Ok(human_addr)
    }
}

/// Query entry point for reading factory state
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Compute what address an account would have if created
        QueryMsg::ComputeAccountAddress { request } => {
            let salt = execute::compute_salt(&env, &request);
            let code_id = JIT_ACCOUNT_CODE_ID.load(deps.storage)?;
            let account =
                execute::compute_instantiate2_address(&deps, &env.contract.address, code_id, &salt)
                    .map_err(|e| StdError::generic_err(e.to_string()))?;

            to_json_binary(&ComputeAccountAddressResponse { 
                account: account.to_string() 
            })
        }

        // Check if a specific account has been created by this factory
        QueryMsg::IsAccountCreated { account } => {
            let addr = deps.api.addr_validate(&account)?;
            let created = CREATED_ACCOUNTS.has(deps.storage, addr);
            to_json_binary(&created)
        }

        // Check if an account request ID has been used
        QueryMsg::IsAccountRequestIdUsed {
            controller,
            account_request_id,
        } => {
            let controller_addr = deps.api.addr_validate(&controller)?;
            let used = USED_NONCES.has(deps.storage, (controller_addr, account_request_id));
            to_json_binary(&used)
        }

        // Get the maximum allowed block age
        QueryMsg::GetMaxBlockAge {} => to_json_binary(&MAX_BLOCK_AGE),
    }
}
