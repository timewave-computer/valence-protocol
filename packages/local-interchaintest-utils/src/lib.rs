// Path of all valence contracts
pub const VALENCE_ARTIFACTS_PATH: &str = "artifacts";
// We keep the polytone contracts here for our tests
pub const POLYTONE_ARTIFACTS_PATH: &str = "local-interchaintest/polytone_contracts";
// When spinning up local-ic, this is where the logs files will be stored, we used this to cache code_ids for a specific local-ic instance
pub const LOGS_FILE_PATH: &str = "local-interchaintest/configs/logs.json";

pub const LOCAL_CODE_ID_CACHE_PATH_NEUTRON: &str =
    "local-interchaintest/code_id_cache_neutron.json";
pub const LOCAL_CODE_ID_CACHE_PATH_JUNO: &str = "local-interchaintest/code_id_cache_juno.json";
pub const GAS_FLAGS: &str = "--gas=auto --gas-adjustment=3.0";

pub mod polytone;
