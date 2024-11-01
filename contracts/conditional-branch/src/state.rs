use crate::msg::ExecuteMsg;
use cw_storage_plus::Map;

pub const ICQ_QUERIES: Map<String, ExecuteMsg> = Map::new("icq_queries");
