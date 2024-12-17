use cw_storage_plus::{Item, Map};
use valence_middleware_utils::broker::types::Broker;

pub const ACTIVE_REGISTRIES: Map<String, Broker> = Map::new("active_registries");
pub const LATEST: Item<String> = Item::new("latest");
