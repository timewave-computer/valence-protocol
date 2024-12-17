use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use neutron_sdk::bindings::types::InterchainQueryResult;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    SetLatestRegistry { version: String, address: String },
}

#[cw_serde]
pub enum QueryMsg {
    DecodeProto {
        registry_version: Option<String>,
        query_id: String,
        icq_result: InterchainQueryResult,
    },
    GetKVKey {
        registry_version: Option<String>,
        params: BTreeMap<String, Binary>,
    },
    ToCanonical {},
    FromCanonical {},
}

#[cw_serde]
pub struct Broker {
    // address of the instantiated registry
    pub registry_address: Addr,
    // semver
    pub version: String,
}
