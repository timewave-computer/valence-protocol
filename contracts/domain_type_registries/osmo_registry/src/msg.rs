use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::StdResult;
use neutron_sdk::bindings::types::KVKey;
use serde_json::Value;
use valence_icq_lib_utils::{define_registry_types, QueryReconstructionRequest};

pub trait QueryTypeDefinition {
    const REPLY_ID: u64;

    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey>;
    fn decode_and_reconstruct(request: &QueryReconstructionRequest) -> StdResult<Value>;
}

define_registry_types! {
    (GammV1Beta1Pool, osmosis_std::types::osmosis::gamm::v1beta1::Pool),
    (BankV1Beta1Balance, osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse)
}
