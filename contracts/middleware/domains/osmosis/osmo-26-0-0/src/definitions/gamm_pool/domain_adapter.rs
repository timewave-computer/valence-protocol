use osmosis_std::types::osmosis::gamm::v1beta1::Pool;

use std::collections::BTreeMap;

use cosmwasm_std::{to_json_binary, Binary, StdError};
use neutron_sdk::bindings::types::{InterchainQueryResult, KVKey};
use osmosis_std::shim::Any;
use valence_middleware_utils::{try_unpack_domain_specific_value, IcqIntegration, MiddlewareError};

use prost::Message;

use super::{OsmosisXykPool, STORAGE_PREFIX};

impl IcqIntegration for OsmosisXykPool {
    fn get_kv_key(&self, params: BTreeMap<String, Binary>) -> Result<KVKey, MiddlewareError> {
        let pool_prefix_key: u8 = 0x02;

        let id: u64 = try_unpack_domain_specific_value("pool_id", &params)?;

        let mut pool_access_key = vec![pool_prefix_key];
        pool_access_key.extend_from_slice(&id.to_be_bytes());

        Ok(KVKey {
            path: STORAGE_PREFIX.to_string(),
            key: Binary::new(pool_access_key),
        })
    }

    fn decode_and_reconstruct(
        query_id: String,
        icq_result: InterchainQueryResult,
    ) -> Result<Binary, MiddlewareError> {
        let any_msg: Any = Any::decode(icq_result.kv_results[0].value.as_slice())
            .map_err(|e| StdError::generic_err(e.to_string()))?;

        let osmo_pool: Pool = any_msg
            .try_into()
            .map_err(|_| StdError::generic_err("failed to decode pool from any"))?;

        let binary = to_json_binary(&osmo_pool)?;

        Ok(binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_json, to_json_binary, Binary};
    use neutron_sdk::bindings::types::StorageValue;
    use osmosis_std::types::osmosis::gamm::v1beta1::{Pool, PoolAsset, PoolParams};

    #[test]
    fn test_get_kv_key() {
        let pool = Pool::default();
        let mut params = BTreeMap::new();
        params.insert("pool_id".to_string(), to_json_binary(&1u64).unwrap());

        let kv_key = OsmosisXykPool(pool).get_kv_key(params).unwrap();
        let b64_key = "AgAAAAAAAAAB";
        let binary_key = Binary::from_base64(b64_key).unwrap();

        assert_eq!(kv_key.path, "gamm");
        assert_eq!(kv_key.key, Binary::new(vec![2, 0, 0, 0, 0, 0, 0, 0, 1]));
        assert_eq!(kv_key.key, binary_key);
    }

    #[test]
    fn test_decode_and_reconstruct() {
        let b64_key = "AgAAAAAAAAAB";
        let binary_key = Binary::from_base64(b64_key).unwrap();

        let b64_value = "Chovb3Ntb3Npcy5nYW1tLnYxYmV0YTEuUG9vbBKGAgo/b3NtbzE5ZTJtZjdjeXdrdjd6YXVnNm5rNWY4N2QwN2Z4cmRncmxhZHZ5bWgyZ3d2NWNydm0zdm5zdWV3aGg3EAEaBgoBMBIBMCIEMTI4aCokCgtnYW1tL3Bvb2wvMRIVMTAwMDAwMDAwMDAwMDAwMDAwMDAwMl8KUQpEaWJjLzRFNDFFRDhGM0RDQUVBMTVGNEQ2QURDNkVERDdDMDRBNjc2MTYwNzM1Qzk3MTBCOTA0QjdCRjUzNTI1QjU2RDYSCTEwMDAwMDAwMBIKMTA3Mzc0MTgyNDIgChIKBXVvc21vEgkxMDAwMDAwMDASCjEwNzM3NDE4MjQ6CjIxNDc0ODM2NDg=";
        let binary_value = Binary::from_base64(b64_value).unwrap();

        let storage_value = StorageValue {
            storage_prefix: "gamm".to_string(),
            key: binary_key,
            value: binary_value,
        };

        let osmo_pool_binary = OsmosisXykPool::decode_and_reconstruct(
            Pool::TYPE_URL.to_string(),
            InterchainQueryResult {
                kv_results: vec![storage_value],
                height: 1,
                revision: 1,
            },
        )
        .unwrap();

        let osmo_pool: Pool = from_json(osmo_pool_binary).unwrap();

        assert_eq!(
            osmo_pool.address,
            "osmo19e2mf7cywkv7zaug6nk5f87d07fxrdgrladvymh2gwv5crvm3vnsuewhh7".to_string()
        );
        assert_eq!(osmo_pool.id, 1);
        assert_eq!(
            osmo_pool.pool_params,
            Some(PoolParams {
                swap_fee: "0".to_string(),
                exit_fee: "0".to_string(),
                smooth_weight_change_params: None
            })
        );
        assert_eq!(osmo_pool.future_pool_governor, "128h".to_string());
        assert_eq!(
            osmo_pool.total_shares,
            Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                denom: "gamm/pool/1".to_string(),
                amount: "100000000000000000000".to_string()
            })
        );
        assert_eq!(osmo_pool.pool_assets.len(), 2);
        assert_eq!(
            osmo_pool.pool_assets[0],
            PoolAsset {
                token: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                    denom: "ibc/4E41ED8F3DCAEA15F4D6ADC6EDD7C04A676160735C9710B904B7BF53525B56D6"
                        .to_string(),
                    amount: "100000000".to_string()
                }),
                weight: "1073741824".to_string()
            }
        );
        assert_eq!(
            osmo_pool.pool_assets[1],
            PoolAsset {
                token: Some(osmosis_std::types::cosmos::base::v1beta1::Coin {
                    denom: "uosmo".to_string(),
                    amount: "100000000".to_string()
                }),
                weight: "1073741824".to_string()
            }
        );
        assert_eq!(osmo_pool.total_weight, "2147483648");
    }
}
