use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use neutron_sdk::bindings::types::InterchainQueryResult;
use valence_middleware_utils::type_registry::types::{NativeTypeWrapper, ValenceType};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    SetLatestRegistry { version: String, address: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(NativeTypeWrapper)]
    DecodeProto {
        registry_version: Option<String>,
        query_id: String,
        icq_result: InterchainQueryResult,
    },
    #[returns(neutron_sdk::bindings::types::KVKey)]
    GetKVKey {
        registry_version: Option<String>,
        query_id: String,
        params: BTreeMap<String, Binary>,
    },
    #[returns(ValenceType)]
    ToCanonical {},
    #[returns(Binary)]
    FromCanonical {},
}

#[cw_serde]
pub struct TypeRegistry {
    // address of the instantiated registry
    pub registry_address: Addr,
    // semver
    pub version: String,
}
