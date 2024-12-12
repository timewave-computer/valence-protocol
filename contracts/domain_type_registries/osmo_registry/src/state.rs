use cw_storage_plus::Item;

// connection id from home domain to osmosis
pub const CONNECTION_ID: Item<String> = Item::new("connection_id");
