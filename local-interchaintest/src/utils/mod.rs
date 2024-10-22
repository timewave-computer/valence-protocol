pub mod authorization;
pub mod base_account;
pub mod manager;
pub mod polytone;
pub mod processor;

// Local-ic chain configs files
pub const NEUTRON_CONFIG_FILE: &str = "neutron.json";
pub const NEUTRON_JUNO_CONFIG_FILE: &str = "neutron_juno.json";

// Path of all valence contracts
pub const VALENCE_ARTIFACTS_PATH: &str = "artifacts";
// We keep the polytone contracts here for our tests
pub const POLYTONE_ARTIFACTS_PATH: &str = "local-interchaintest/polytone_contracts";
// Where we are keeping the astroport contracts for all our tests
pub const ASTROPORT_PATH: &str = "packages/astroport-utils/contracts";
// When spinning up local-ic, this is where the logs files will be stored, we used this to cache code_ids for a specific local-ic instance
pub const LOGS_FILE_PATH: &str = "local-interchaintest/configs/logs.json";

pub const LOCAL_CODE_ID_CACHE_PATH_NEUTRON: &str =
    "local-interchaintest/code_id_cache_neutron.json";
pub const LOCAL_CODE_ID_CACHE_PATH_JUNO: &str = "local-interchaintest/code_id_cache_juno.json";
pub const GAS_FLAGS: &str = "--gas=auto --gas-adjustment=3.0";
pub const NTRN_DENOM: &str = "untrn";

pub const NEUTRON_USER_ADDRESS_1: &str = "neutron1kljf09rj77uxeu5lye7muejx6ajsu55cuw2mws";
pub const USER_KEY_1: &str = "acc1";

pub const ASTROPORT_LP_SUBDENOM: &str = "astroport/share";
