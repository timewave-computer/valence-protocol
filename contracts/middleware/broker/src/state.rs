use cw_storage_plus::{Item, Map};

use crate::msg::TypeRegistry;

// semver -> TypeRegistry
pub const ACTIVE_REGISTRIES: Map<String, TypeRegistry> = Map::new("active_registries");
pub const LATEST: Item<String> = Item::new("latest");
