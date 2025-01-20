use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;
use valence_middleware_utils::type_registry::types::ValenceType;

pub const ASSOCIATED_QUERY_IDS: Map<u64, PendingQueryIdConfig> = Map::new("associated_query_ids");
pub const QUERY_RESULTS: Map<u64, ValenceType> = Map::new("query_results");

#[cw_serde]
pub struct PendingQueryIdConfig {
    pub type_url: String,
    pub broker_addr: String,
    pub registry_version: Option<String>,
    pub storage_acc: Addr,
}
