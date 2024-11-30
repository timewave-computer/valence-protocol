use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdError, StdResult};
use neutron_sdk::{
    bindings::types::KVKey,
    interchain_queries::{
        helpers::decode_and_convert,
        v047::{helpers::create_account_denom_balance_key, types::BANK_STORE_KEY},
    },
};
use serde_json::Value;
use valence_icq_lib_utils::{BankResultTypes, GammResultTypes, QueryResult};

use crate::error::ContractError;

pub trait QueryTypeDefinition {
    const REPLY_ID: u64;

    fn get_query_type(&self) -> QueryResult;
    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey>;
}

macro_rules! define_osmosis_types {
    ($(($variant:ident, $type:ty)),* $(,)?) => {
        #[allow(clippy::large_enum_variant)]
        #[cw_serde]
        pub enum OsmosisTypes {
            $(
                $variant($type),
            )*
        }

        impl FromStr for OsmosisTypes
        where $($type: QueryTypeDefinition),*
        {
            type Err = ContractError;

            fn from_str(type_url: &str) -> Result<Self, Self::Err> {
                match type_url {
                    $(
                        <$type>::TYPE_URL => Ok(OsmosisTypes::$variant(<$type>::default())),
                    )*
                    _ => Err(ContractError::UnknownTypeUrl(type_url.to_string())),
                }
            }
        }

        impl OsmosisTypes {
                    pub fn get_registration_config(&self, params: serde_json::Map<String, Value>) -> StdResult<(KVKey, u64, QueryResult)> {
                        match self {
                            $(
                                OsmosisTypes::$variant(t) => Ok((
                                    t.get_kv_key(params)?,
                                    <$type>::REPLY_ID,
                                    t.get_query_type()
                                )),
                            )*
                        }
                    }
                }
    };
}

define_osmosis_types! {
    (GammV1Beta1Pool, osmosis_std::types::osmosis::gamm::v1beta1::Pool),
    (BankV1Beta1Balance, osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse)
}

impl QueryTypeDefinition for osmosis_std::types::osmosis::gamm::v1beta1::Pool {
    const REPLY_ID: u64 = 31415;

    fn get_query_type(&self) -> QueryResult {
        QueryResult::Gamm {
            result_type: GammResultTypes::Pool,
        }
    }

    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey> {
        let pool_prefix_key: u8 = 0x02;
        let pool_id = match params["pool_id"].clone() {
            Value::Number(number) => match number.as_u64() {
                Some(n) => n,
                None => {
                    return Err(StdError::generic_err(format!(
                        "failed to parse {:?} as u64 for pool_id access",
                        number
                    )))
                }
            },
            Value::String(str_num) => str_num.parse::<u64>().map_err(|_| {
                StdError::generic_err(format!("failed to parse pool_id {:?} to u64", str_num))
            })?,
            _ => {
                return Err(StdError::generic_err(format!(
                    "field pool_id missing from query params: {:?}",
                    params
                )))
            }
        };

        let mut pool_access_key = vec![pool_prefix_key];
        pool_access_key.extend_from_slice(&pool_id.to_be_bytes());

        Ok(KVKey {
            path: "gamm".to_string(),
            key: Binary::new(pool_access_key),
        })
    }
}

impl QueryTypeDefinition for osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse {
    const REPLY_ID: u64 = 31416;

    fn get_query_type(&self) -> QueryResult {
        QueryResult::Bank {
            result_type: BankResultTypes::AccountDenomBalance,
        }
    }

    fn get_kv_key(&self, params: serde_json::Map<String, Value>) -> StdResult<KVKey> {
        // let addr = params["addr"].to_string();

        let addr = match params["addr"].clone() {
            Value::String(address_str) => address_str,
            _ => {
                return Err(StdError::generic_err(format!(
                    "field addr missing from query params: {:?}",
                    params
                )))
            }
        };

        let denom = match params["denom"].clone() {
            Value::String(denom_str) => denom_str,
            _ => {
                return Err(StdError::generic_err(format!(
                    "field denom missing from query params: {:?}",
                    params
                )))
            }
        };

        // let denom = params["denom"].to_string();

        let converted_addr_bytes = decode_and_convert(&addr).unwrap();

        let balance_key = create_account_denom_balance_key(converted_addr_bytes, denom).unwrap();

        Ok(KVKey {
            path: BANK_STORE_KEY.to_string(),
            key: Binary::new(balance_key),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_balance_get_kv_key() {
        let balance = osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse::default();
        let params = json!({
            "addr": "osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63",
            "denom": "uosmo"
        });

        let param_map = params.as_object().unwrap();

        let result = balance.get_kv_key(param_map.clone()).unwrap();
    }

    #[test]
    fn test_json_map_serde() {
        let params = json!({
            "pool_id": "1"
        });

        let json_restored: serde_json::Map<String, Value> = params.as_object().unwrap().clone();
        println!("json restored: {:?}", json_restored);

        let pool_id = json_restored["pool_id"].as_u64().unwrap();

        println!("pool_id: {:?}", pool_id);
    }

    #[test]
    fn test_gamm_pool_get_kv_key() {
        let pool = osmosis_std::types::osmosis::gamm::v1beta1::Pool::default();
        let params = json!({
            "pool_id": 1
        });

        let param_str = params.to_string();

        let json_restored: Value = serde_json::from_str(&param_str).unwrap();
        println!("json restored: {:?}", json_restored);

        let pool_id = json_restored["pool_id"].as_u64().unwrap();

        println!("pool_id: {:?}", pool_id);

        let param_map = params.as_object().unwrap();

        let result = pool.get_kv_key(param_map.clone()).unwrap();

        assert_eq!(result.path, "gamm");
        let expected_key = {
            let mut key = vec![0x02];
            key.extend_from_slice(&1u64.to_be_bytes());
            key
        };
        assert_eq!(result.key.as_slice(), expected_key.as_slice());
    }
}
