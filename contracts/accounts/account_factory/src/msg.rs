// Purpose: Message types for account factory contract
use cosmwasm_schema::{cw_serde, QueryResponses};

pub const MAX_BLOCK_AGE: u64 = 200;

#[cw_serde]
pub struct InstantiateMsg {
    pub fee_collector: Option<String>,
    pub jit_account_code_id: u64,
}

#[cw_serde]
pub struct AccountRequest {
    pub controller: String,
    pub libraries: Vec<String>,
    pub program_id: String,
    pub account_request_id: u64,
    pub historical_block_height: u64, // Block height used for entropy
    pub signature: Option<Vec<u8>>,   // Optional for atomic operations
    pub public_key: Option<Vec<u8>>, // Required when signature is provided (33 bytes for compressed secp256k1)
}

/// Account request data used for signature verification (excludes signature field)
#[cw_serde]
pub struct AccountRequestForSigning {
    pub controller: String,
    pub libraries: Vec<String>,
    pub program_id: String,
    pub account_request_id: u64,
    pub historical_block_height: u64,
}

impl From<&AccountRequest> for AccountRequestForSigning {
    fn from(request: &AccountRequest) -> Self {
        Self {
            controller: request.controller.clone(),
            libraries: request.libraries.clone(),
            program_id: request.program_id.clone(),
            account_request_id: request.account_request_id,
            historical_block_height: request.historical_block_height,
        }
    }
}

#[cw_serde]
pub struct BatchRequest {
    pub requests: Vec<AccountRequest>,
    pub ferry: String,
    pub fee_amount: u128,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Create a single account with historical block validation
    CreateAccount { request: AccountRequest },
    /// Create account and process request atomically
    CreateAccountWithRequest { request: AccountRequest },
    /// Process multiple account creations in batch
    CreateAccountsBatch { batch: BatchRequest },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Compute the deterministic address for an account
    #[returns(ComputeAccountAddressResponse)]
    ComputeAccountAddress { request: AccountRequest },
    /// Check if an account has been created
    #[returns(bool)]
    IsAccountCreated { account: String },
    /// Check if an account request ID has been used
    #[returns(bool)]
    IsAccountRequestIdUsed {
        controller: String,
        account_request_id: u64,
    },
    /// Get the maximum allowed block age
    #[returns(u64)]
    GetMaxBlockAge {},
}

#[cw_serde]
pub struct ComputeAccountAddressResponse {
    pub account: String,
}

#[cw_serde]
pub struct JitAccountInstantiateMsg {
    pub controller: String,
}
