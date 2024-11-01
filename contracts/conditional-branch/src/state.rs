use cw_storage_plus::Map;

pub const ICQ_QUERIES: Map<u64, Option<u64>> = Map::new("icq_queries");
