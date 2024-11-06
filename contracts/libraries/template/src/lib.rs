use cw_storage_plus::Item;
use msg::Config2;

pub mod contract;
pub mod msg;

#[cfg(test)]
mod tests;

pub(crate) const CONFIG2: Item<Config2> = Item::new("config2");
