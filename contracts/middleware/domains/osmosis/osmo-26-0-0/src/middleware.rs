use std::collections::BTreeMap;

use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{from_json, StdError, StdResult};

// fn process_pool<T: Into<ValenceXykPool>>(external_pool: T) {
//     // let valence_pool = external_pool.to_valence_xyk_pool();
// }

// trait ValenceMiddlewareTypeDefinition {
//     fn serialize(&self) -> Binary;
//     fn deserialize(&self) -> Self;
// }

pub fn try_unpack_domain_specific_value<T>(
    key: &str,
    domain_specific_fields: &BTreeMap<String, cosmwasm_std::Binary>,
) -> StdResult<T>
where
    T: DeserializeOwned,
{
    let binary = domain_specific_fields
        .get(key)
        .ok_or(StdError::generic_err(format!(
            "failed to get {} field from domain specific fields",
            key
        )))
        .unwrap();

    from_json(binary)
}
