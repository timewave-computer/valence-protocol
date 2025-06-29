// Purpose: Constants for account factory e2e examples

use std::time::Duration;

/// Default timeout for network operations  
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Test mnemonics for local testing
pub const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// Default gas limits
pub const DEFAULT_GAS_LIMIT: u64 = 300_000;

/// Default block heights for historical validation
pub const HISTORICAL_BLOCK_HEIGHT: u64 = 12345;

/// Ferry service defaults
pub const DEFAULT_BATCH_SIZE: usize = 3;
pub const DEFAULT_FEE_PER_REQUEST: u128 = 1000;

/// Default chain IDs for testing
pub const ETH_CHAIN_ID: u64 = 31337; // Anvil default
pub const NEUTRON_CHAIN_ID: &str = "test-1";

/// Status tracking
#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    Pending,
    Processing,
    AccountsCreated,
    Failed,
}

/// Test environment URLs
pub mod urls {
    pub const DEFAULT_ANVIL_RPC: &str = "http://127.0.0.1:8545";
    pub const DEFAULT_COSMWASM_RPC: &str = "http://127.0.0.1:26657";
    pub const DEFAULT_COPROCESSOR: &str = "http://127.0.0.1:37281";
} 