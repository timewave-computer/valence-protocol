use cosmwasm_schema::serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use thiserror::Error;

use cosmwasm_std::{from_json, Binary, StdError, StdResult};
use neutron_sdk::{
    bindings::types::{InterchainQueryResult, KVKey},
    NeutronError,
};

pub mod canonical_types;
pub mod type_registry;

pub trait IcqIntegration {
    fn get_kv_key(params: BTreeMap<String, Binary>) -> Result<KVKey, MiddlewareError>;
    fn decode_and_reconstruct(
        query_id: String,
        icq_result: InterchainQueryResult,
    ) -> Result<Binary, MiddlewareError>;
}

#[derive(Error, Debug)]
pub enum MiddlewareError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("failed to decode proto: {0}")]
    DecodeError(String),

    #[error("{0}")]
    NeutronError(#[from] NeutronError),

    #[error("{0}")]
    SemverError(#[from] semver::Error),
}

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
