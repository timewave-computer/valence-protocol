use cosmwasm_std::{from_base64, from_json, to_json_string, Binary, StdError, StdResult};
use neutron_sdk::{
    bindings::types::KVKey,
    interchain_queries::{
        helpers::decode_and_convert,
        types::KVReconstruct,
        v047::{helpers::create_account_denom_balance_key, types::BANK_STORE_KEY},
    },
};
use osmosis_std::{shim::Any, types::cosmos::bank::v1beta1::QueryBalanceResponse};
use prost::Message;
use serde_json::Value;
use valence_icq_lib_utils::{get_str_query_param, QueryReconstructionRequest};

use crate::msg::QueryTypeDefinition;

impl QueryTypeDefinition for osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse {
    const REPLY_ID: u64 = 31416;

    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey> {
        let addr = get_str_query_param(&params, "addr")?;
        let denom = get_str_query_param(&params, "denom")?;

        let converted_addr_bytes = decode_and_convert(&addr)
            .map_err(|_| StdError::generic_err("failed to decode addr"))?;
        let balance_key = create_account_denom_balance_key(converted_addr_bytes, denom).unwrap();

        Ok(KVKey {
            path: BANK_STORE_KEY.to_string(),
            key: Binary::new(balance_key),
        })
    }

    fn decode_and_reconstruct(request: &QueryReconstructionRequest) -> StdResult<Value> {
        let balances: neutron_sdk::interchain_queries::v047::types::Balances =
            KVReconstruct::reconstruct(&request.icq_result.kv_results).map_err(|e| {
                StdError::generic_err(format!("failed to reconstruct query result: {:?}", e))
            })?;

        let balances_str = to_json_string(&balances)?;

        let json_value: Value = serde_json::from_str(&balances_str)
            .map_err(|_e| StdError::generic_err("failed to obtain value from str"))?;

        Ok(json_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neutron_sdk::bindings::types::StorageValue;
    use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;
    use serde_json::json;

    #[test]
    fn test_get_kv_key() {
        let mut params = serde_json::Map::new();
        params.insert(
            "addr".to_string(),
            json!("osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63"),
        );
        params.insert("denom".to_string(), json!("uosmo"));

        let qb_response = QueryBalanceResponse::default();
        let kvk_response = qb_response.get_kv_key(params).unwrap();

        let key_binary = Binary::from_base64("AhS8qJZnI6YkudXN06CxcRJLTC+8wXVvc21v").unwrap();

        assert_eq!(kvk_response.path, "bank".to_string());
        assert_eq!(kvk_response.key, key_binary);
    }

    #[test]
    fn test_decode_and_reconstruct() {
        let key_binary = Binary::from_base64("AhS8qJZnI6YkudXN06CxcRJLTC+8wXVvc21v").unwrap();
        let value_binary = Binary::from_base64("OTk5ODg5OTk5NzUwMA==").unwrap();

        let request = QueryReconstructionRequest {
            icq_result: neutron_sdk::bindings::types::InterchainQueryResult {
                kv_results: vec![StorageValue {
                    key: key_binary,
                    value: value_binary,
                    storage_prefix: "bank".to_string(),
                }],
                height: 1,
                revision: 1,
            },
            query_type: QueryBalanceResponse::TYPE_URL.to_string(),
        };

        let result = QueryBalanceResponse::decode_and_reconstruct(&request).unwrap();

        let expected_json = json!({
            "coins": [
                {
                    "amount": "9998899997500",
                    "denom": "uosmo"
                }
            ]
        });
        assert_eq!(result, expected_json);
    }
}
