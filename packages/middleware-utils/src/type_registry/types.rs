use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use neutron_sdk::bindings::types::{InterchainQueryResult, KVKey};

use crate::canonical_types::{bank::balance::ValenceBankBalance, pools::xyk::ValenceXykPool};

#[cw_serde]
pub struct RegistryInstantiateMsg {}
#[cw_serde]
pub enum RegistryExecuteMsg {}

/// this is effectively the valence vm types that are enabled
/// on a system level. if a particular type is not defined here,
/// it cannot be used in programs.
#[cw_serde]
pub enum ValenceType {
    XykPool(ValenceXykPool),
    BankBalance(ValenceBankBalance),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum RegistryQueryMsg {
    /// serialize a message to binary
    #[returns(NativeTypeWrapper)]
    FromCanonical { obj: ValenceType },
    /// deserialize a message from binary/bytes
    #[returns(Binary)]
    ToCanonical { type_url: String, binary: Binary },

    /// get the kvkey used for registering an interchain query
    #[returns(KVKey)]
    KVKey {
        type_id: String,
        params: BTreeMap<String, Binary>,
    },

    #[returns(NativeTypeWrapper)]
    ReconstructProto {
        query_id: String,
        icq_result: InterchainQueryResult,
    },
    // TODO: transform an outdated type to a new version
}

#[cw_serde]
pub struct NativeTypeWrapper {
    pub binary: Binary,
}
