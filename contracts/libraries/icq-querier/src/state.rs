use crate::msg::QueryResult;
use cosmwasm_schema::cw_serde;
use cw_storage_plus::Map;
use serde_json::Value;

pub const LOGS: Map<String, String> = Map::new("logs");
pub const ASSOCIATED_QUERY_IDS: Map<u64, PendingQueryIdConfig> = Map::new("associated_query_ids");
pub const QUERY_RESULTS: Map<u64, Value> = Map::new("query_results");

pub const QUERY_REGISTRATION_REPLY_IDS: Map<u64, PendingQueryIdConfig> =
    Map::new("query_registration_reply_ids");

#[cw_serde]
pub struct PendingQueryIdConfig {
    pub associated_domain_registry: String,
    pub query_type: QueryResult,
}
