use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_string, Binary, StdError, StdResult};
use neutron_sdk::{
    bindings::types::KVKey,
    interchain_queries::{
        helpers::decode_and_convert,
        types::KVReconstruct,
        v047::{helpers::create_account_denom_balance_key, types::BANK_STORE_KEY},
    },
};
use osmosis_std::shim::Any;
use prost::Message;
use serde_json::Value;
use valence_icq_lib_utils::{
    define_registry_types, get_str_query_param, get_u64_query_param, QueryReconstructionRequest,
};

use crate::error::ContractError;

pub trait QueryTypeDefinition {
    const REPLY_ID: u64;

    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey>;
    fn decode_and_reconstruct(request: &QueryReconstructionRequest) -> StdResult<Value>;
}

define_registry_types! {
    (GammV1Beta1Pool, osmosis_std::types::osmosis::gamm::v1beta1::Pool),
    (BankV1Beta1Balance, osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse)
}

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
