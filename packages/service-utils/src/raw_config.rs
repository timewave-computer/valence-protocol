use cosmwasm_std::{StdError, StdResult, Storage};
use cw_storage_plus::Item;
use serde::{de::DeserializeOwned, Serialize};

/// Get the Item helper for the raw service config
pub fn get_service_config_item<T: Serialize + DeserializeOwned>() -> Item<T> {
    Item::new("raw_service_config")
}

pub fn load_raw_service_config<T: Serialize + DeserializeOwned>(
    storage: &dyn Storage,
) -> StdResult<T> {
    get_service_config_item::<T>().load(storage)
}

pub fn save_raw_service_config<T: Serialize + DeserializeOwned>(
    storage: &mut dyn Storage,
    config: &T,
) -> StdResult<()> {
    get_service_config_item::<T>().save(storage, config)
}

pub fn update_raw_service_config<T: Serialize + DeserializeOwned, F, E>(
    storage: &mut dyn Storage,
    action: F,
) -> Result<T, E>
where
    F: FnOnce(T) -> Result<T, E>,
    E: From<StdError>,
{
    get_service_config_item::<T>().update(storage, action)
}

pub fn query_raw_service_config<T: Serialize + DeserializeOwned>(
    storage: &dyn Storage,
) -> StdResult<T> {
    load_raw_service_config(storage)
}
