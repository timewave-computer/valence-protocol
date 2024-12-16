use std::collections::BTreeMap;

use cosmwasm_std::{Binary, StdResult};
use neutron_sdk::bindings::types::{InterchainQueryResult, KVKey};
// use valence_canonical_types::pools::xyk::ValenceXykPool;

// pub trait ValenceTypeAdapter {
//     type External;

//     fn to_canonical(&self) -> StdResult<ValenceXykPool>;
//     fn from_canonical(canonical: ValenceXykPool) -> StdResult<Self::External>;
// }

pub trait MiddlewareSerializer: Sized {
    fn serialize(&self) -> StdResult<Binary>;
    fn deserialize(binary: Binary) -> StdResult<Self>;
}

pub trait IcqIntegration {
    fn get_kv_key(&self, params: BTreeMap<String, Binary>) -> StdResult<KVKey>;
    fn decode_and_reconstruct(
        query_id: String,
        icq_result: InterchainQueryResult,
    ) -> StdResult<Binary>;
}
