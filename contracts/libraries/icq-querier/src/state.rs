use cosmwasm_std::Binary;
use cw_storage_plus::Map;
use valence_icq_lib_utils::PendingQueryIdConfig;

pub const ASSOCIATED_QUERY_IDS: Map<u64, PendingQueryIdConfig> = Map::new("associated_query_ids");
pub const QUERY_RESULTS: Map<u64, Binary> = Map::new("query_results");
