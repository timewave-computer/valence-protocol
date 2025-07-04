use std::any::type_name;

use cosmwasm_std::{from_json, to_json_vec, Addr, StdError, StdResult, Storage};
use cw_ownable::Ownership;
use cw_storage_plus::Item;
use serde::{de::DeserializeOwned, Serialize};
use valence_library_utils::raw_config::load_raw_library_config;

pub const CONFIG_KEY: &[u8] = b"config";
pub const PROCESSOR: Item<Addr> = Item::new("processor");

pub fn get_ownership(store: &dyn Storage) -> StdResult<Ownership<Addr>> {
    cw_ownable::get_ownership(store)
}

pub fn get_processor(store: &dyn Storage) -> StdResult<Addr> {
    PROCESSOR.load(store)
}

pub fn save_config<T>(store: &mut dyn Storage, config: &T) -> StdResult<()>
where
    T: Serialize + DeserializeOwned,
{
    store.set(CONFIG_KEY, &to_json_vec(config)?);
    Ok(())
}

pub fn load_config<T>(store: &dyn Storage) -> StdResult<T>
where
    T: Serialize + DeserializeOwned,
{
    if let Some(value) = store.get(CONFIG_KEY) {
        from_json(value)
    } else {
        let object_info = not_found_object_info::<T>(CONFIG_KEY);
        Err(StdError::not_found(object_info))
    }
}

pub fn load_raw_config<T>(store: &dyn Storage) -> StdResult<T>
where
    T: Serialize + DeserializeOwned,
{
    load_raw_library_config(store)
}

fn not_found_object_info<T>(key: &[u8]) -> String {
    let type_name = type_name::<T>();
    format!("type: {type_name}; key: {key:02X?}")
}
