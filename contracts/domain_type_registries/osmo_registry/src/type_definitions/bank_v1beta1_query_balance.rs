use cosmwasm_std::{to_json_string, Binary, StdError, StdResult};
use neutron_sdk::{
    bindings::types::KVKey,
    interchain_queries::{
        helpers::decode_and_convert,
        types::KVReconstruct,
        v047::{helpers::create_account_denom_balance_key, types::BANK_STORE_KEY},
    },
};
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
