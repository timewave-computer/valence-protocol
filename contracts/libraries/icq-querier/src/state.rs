use cw_storage_plus::Map;
use serde_json::Value;
use valence_icq_lib_utils::PendingQueryIdConfig;

pub const LOGS: Map<String, String> = Map::new("logs");
pub const ASSOCIATED_QUERY_IDS: Map<u64, PendingQueryIdConfig> = Map::new("associated_query_ids");
pub const QUERY_RESULTS: Map<u64, Value> = Map::new("query_results");
