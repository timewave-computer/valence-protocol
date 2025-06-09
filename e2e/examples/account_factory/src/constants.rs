// Purpose: Constants for Account Factory E2E tests

/// Default Anvil RPC URL for local EVM testing
pub const LOCAL_ANVIL_RPC_URL: &str = "http://127.0.0.1:8545";

/// Default CosmWasm chain RPC URL for testing
pub const LOCAL_COSMWASM_RPC_URL: &str = "http://127.0.0.1:26657";

/// Default ZK Coprocessor URL for testing
pub const LOCAL_COPROCESSOR_URL: &str = "http://127.0.0.1:37281";

/// Maximum timeout for API responses in seconds
pub const MAX_API_RESPONSE_TIME_SECONDS: u64 = 30;

/// Environment variable names
pub const ENV_ANVIL_RPC_URL: &str = "ANVIL_RPC_URL";
pub const ENV_COSMWASM_RPC_URL: &str = "COSMWASM_RPC_URL";
pub const ENV_COPROCESSOR_URL: &str = "COPROCESSOR_URL";
pub const ENV_E2E_MNEMONIC: &str = "E2E_MNEMONIC";

/// Test mnemonic for development
pub const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// Account type constants
pub const ACCOUNT_TYPE_TOKEN_CUSTODY: u8 = 1;
pub const ACCOUNT_TYPE_DATA_STORAGE: u8 = 2;
pub const ACCOUNT_TYPE_HYBRID: u8 = 3;

/// Test contract addresses (mocked)
pub const MOCK_EVM_FACTORY: &str = "0x2222222222222222222222222222222222222222";
pub const MOCK_EVM_IMPLEMENTATION: &str = "0x1111111111111111111111111111111111111111";
pub const MOCK_COSMWASM_FACTORY: &str = "cosmos1factoryaddress123456789abcdef0123456789";
pub const MOCK_JIT_CODE_ID: u64 = 123; 