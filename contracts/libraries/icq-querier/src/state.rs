use crate::msg::QueryResult;
use cw_storage_plus::Map;
use serde_json::Value;

pub const LOGS: Map<String, String> = Map::new("logs");
pub const ASSOCIATED_QUERY_IDS: Map<u64, QueryResult> = Map::new("associated_query_ids");

pub const QUERY_RESULTS: Map<u64, Value> = Map::new("query_results");
