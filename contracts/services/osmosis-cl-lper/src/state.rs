use cw_storage_plus::Item;

use crate::msg::Config;

pub const PENDING_CTX: Item<Config> = Item::new("pending_reply");
