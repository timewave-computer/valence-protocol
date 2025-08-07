use cosmwasm_std::Addr;
use cw_storage_plus::Map;

// Routes to different verifier contracts
pub const ROUTES: Map<String, Addr> = Map::new("routes");
