pub mod authorization;
pub mod base_account;
pub mod hyperlane;
pub mod ibc;
pub mod icq;
pub mod manager;
pub mod osmosis;
pub mod persistence;
pub mod polytone;
pub mod processor;
pub mod relayer;
pub mod solidity_contracts;

// Local-ic chain configs files
pub const NEUTRON_CONFIG_FILE: &str = "neutron.json";
pub const NEUTRON_JUNO_CONFIG_FILE: &str = "neutron_juno.json";
pub const NEUTRON_OSMO_CONFIG_FILE: &str = "neutron_osmosis.json";

// Path of all valence contracts
pub const VALENCE_ARTIFACTS_PATH: &str = "artifacts";
// We keep the polytone contracts here for our tests
pub const POLYTONE_ARTIFACTS_PATH: &str = "local-interchaintest/polytone_contracts";
// We keep the cosmwasm hyperlane contracts here to set up hyperlane on a cosmwasm chain
pub const HYPERLANE_COSMWASM_ARTIFACTS_PATH: &str =
    "local-interchaintest/hyperlane/contracts/cosmwasm";
pub const HYPERLANE_RELAYER_CONFIG_PATH: &str = "local-interchaintest/hyperlane/config/config.json";
// Where we are keeping the astroport contracts for all our tests
pub const ASTROPORT_PATH: &str = "packages/astroport-utils/contracts";
// When spinning up local-ic, this is where the logs files will be stored, we used this to cache code_ids for a specific local-ic instance
pub const LOGS_FILE_PATH: &str = "local-interchaintest/configs/logs.json";

pub const LOCAL_CODE_ID_CACHE_PATH_NEUTRON: &str =
    "local-interchaintest/code_id_cache_neutron.json";
pub const LOCAL_CODE_ID_CACHE_PATH_JUNO: &str = "local-interchaintest/code_id_cache_juno.json";
pub const LOCAL_CODE_ID_CACHE_PATH_PERSISTENCE: &str =
    "local-interchaintest/code_id_cache_persistence.json";
pub const LOCAL_CODE_ID_CACHE_PATH_OSMOSIS: &str =
    "local-interchaintest/code_id_cache_osmosis.json";

pub const GAS_FLAGS: &str = "--gas=auto --gas-adjustment=3.0";
pub const NTRN_DENOM: &str = "untrn";

pub const NEUTRON_USER_ADDRESS_1: &str = "neutron1kljf09rj77uxeu5lye7muejx6ajsu55cuw2mws";
pub const USER_KEY_1: &str = "acc1";

pub const PERSISTENCE_CHAIN_DENOM: &str = "uxrpt";
pub const PERSISTENCE_CHAIN_ID: &str = "localpersistence-1";
pub const PERSISTENCE_CHAIN_NAME: &str = "persistence";
pub const PERSISTENCE_CHAIN_PREFIX: &str = "persistence";
pub const PERSISTENCE_CHAIN_ADMIN_ADDR: &str = "persistence1hj5fveer5cjtn4wd6wstzugjfdxzl0xpgq5pz8";

pub const ASTROPORT_LP_SUBDENOM: &str = "astroport/share";

pub const DEFAULT_ANVIL_RPC_ENDPOINT: &str = "http://localhost:8545";
