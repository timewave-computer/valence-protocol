// Purpose: Message types for account factory contract
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

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
    pub account: Addr,
}

#[cw_serde]
pub struct JitAccountInstantiateMsg {
    pub controller: String,
}
