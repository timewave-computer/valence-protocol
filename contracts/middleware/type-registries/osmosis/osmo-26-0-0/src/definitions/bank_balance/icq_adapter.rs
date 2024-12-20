use std::collections::BTreeMap;

use cosmwasm_std::{to_json_binary, Binary};
use neutron_sdk::{
    bindings::types::{InterchainQueryResult, KVKey},
    interchain_queries::{
        helpers::decode_and_convert,
        types::KVReconstruct,
        v047::{
            helpers::create_account_denom_balance_key,
            types::{Balances, BANK_STORE_KEY},
        },
    },
};
use valence_middleware_utils::{try_unpack_domain_specific_value, IcqIntegration, MiddlewareError};

use super::{OsmosisBankBalance, ADDR_KEY, DENOM_KEY};

impl IcqIntegration for OsmosisBankBalance {
    fn get_kv_key(params: BTreeMap<String, Binary>) -> Result<KVKey, MiddlewareError> {
        let addr: String = try_unpack_domain_specific_value(ADDR_KEY, &params)?;
        let denom: String = try_unpack_domain_specific_value(DENOM_KEY, &params)?;

        let converted_addr_bytes = decode_and_convert(&addr)?;

        let balance_key = create_account_denom_balance_key(converted_addr_bytes, denom).unwrap();

        Ok(KVKey {
            path: BANK_STORE_KEY.to_string(),
            key: Binary::new(balance_key),
        })
    }

    fn decode_and_reconstruct(
        _query_id: String,
        icq_result: InterchainQueryResult,
    ) -> Result<Binary, MiddlewareError> {
        let balances: Balances = KVReconstruct::reconstruct(&icq_result.kv_results)?;

        Ok(to_json_binary(&balances)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;
    use neutron_sdk::bindings::types::StorageValue;
    use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;

    #[test]
    fn test_get_kv_key() {
        let mut params = BTreeMap::new();

        params.insert(
            ADDR_KEY.to_string(),
            to_json_binary(&"osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63").unwrap(),
        );
        params.insert(DENOM_KEY.to_string(), to_json_binary(&"uosmo").unwrap());

        let kvk_response = OsmosisBankBalance::get_kv_key(params).unwrap();

        let key_binary = Binary::from_base64("AhS8qJZnI6YkudXN06CxcRJLTC+8wXVvc21v").unwrap();

        assert_eq!(kvk_response.path, "bank".to_string());
        assert_eq!(kvk_response.key, key_binary);
    }

    #[test]
    fn test_decode_and_reconstruct() {
        let key_binary = Binary::from_base64("AhS8qJZnI6YkudXN06CxcRJLTC+8wXVvc21v").unwrap();
        let value_binary = Binary::from_base64("OTk5ODg5OTk5NzUwMA==").unwrap();

        let icq_result = InterchainQueryResult {
            kv_results: vec![StorageValue {
                key: key_binary,
                value: value_binary,
                storage_prefix: "bank".to_string(),
            }],
            height: 1,
            revision: 1,
        };

        let result = OsmosisBankBalance::decode_and_reconstruct(
            QueryBalanceResponse::TYPE_URL.to_string(),
            icq_result,
        )
        .unwrap();

        let balance_response: Balances = from_json(result).unwrap();

        assert_eq!(balance_response.coins.len(), 1);

        assert_eq!(balance_response.coins[0].denom, "uosmo");
        assert_eq!(balance_response.coins[0].amount.u128(), 9998899997500);
    }
}
