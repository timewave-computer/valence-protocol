use cw_storage_plus::Map;

pub const LOGS: Map<String, String> = Map::new("logs");
pub const ASSOCIATED_QUERY_IDS: Map<u64, String> = Map::new("associated_query_ids");

pub const QUERY_RESULTS: Map<u64, String> = Map::new("query_results");
