use cosmwasm_std::{to_json_string, Binary, StdError, StdResult};
use neutron_sdk::bindings::types::KVKey;
use osmosis_std::shim::Any;
use prost::Message;
use serde_json::Value;
use valence_icq_lib_utils::{get_u64_query_param, QueryReconstructionRequest};

use crate::msg::QueryTypeDefinition;

impl QueryTypeDefinition for osmosis_std::types::osmosis::gamm::v1beta1::Pool {
    const REPLY_ID: u64 = 31415;

    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey> {
        let pool_prefix_key: u8 = 0x02;

        let pool_id = get_u64_query_param(&params, "pool_id")?;

        let mut pool_access_key = vec![pool_prefix_key];
        pool_access_key.extend_from_slice(&pool_id.to_be_bytes());

        Ok(KVKey {
            path: "gamm".to_string(),
            key: Binary::new(pool_access_key),
        })
    }

    fn decode_and_reconstruct(request: &QueryReconstructionRequest) -> StdResult<Value> {
        let any_msg: Any = Any::decode(request.icq_result.kv_results[0].value.as_slice())
            .map_err(|e| StdError::generic_err(e.to_string()))?;

        let osmo_pool: osmosis_std::types::osmosis::gamm::v1beta1::Pool = any_msg
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode pool from any"))?;

        let json_str_pool = to_json_string(&osmo_pool)?;

        let json_value: Value = serde_json::from_str(&json_str_pool)
            .map_err(|_e| StdError::generic_err("failed to obtain value from str"))?;

        Ok(json_value)
    }
}
