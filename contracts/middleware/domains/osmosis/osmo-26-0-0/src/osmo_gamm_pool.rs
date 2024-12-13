pub mod valence_xyk_type {
    use std::collections::BTreeMap;
    use std::str::FromStr;

    use crate::{middleware::try_unpack_domain_specific_value, xyk::ValenceXykPool};
    use cosmwasm_std::to_json_binary;
    use cosmwasm_std::{StdError, Uint128};
    use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
    use osmosis_std::types::osmosis::gamm::v1beta1::PoolParams;
    use osmosis_std::types::{cosmos::base::v1beta1::Coin, osmosis::gamm::v1beta1::PoolAsset};

    impl TryFrom<Pool> for ValenceXykPool {
        type Error = StdError;

        fn try_from(pool: Pool) -> Result<Self, Self::Error> {
            // pack all the domain-specific fields
            let mut domain_specific_fields = BTreeMap::from([
                ("address".to_string(), to_json_binary(&pool.address)?),
                ("id".to_string(), to_json_binary(&pool.id)?),
                (
                    "future_pool_governor".to_string(),
                    to_json_binary(&pool.future_pool_governor)?,
                ),
                (
                    "total_weight".to_string(),
                    to_json_binary(&pool.total_weight)?,
                ),
                (
                    "pool_params".to_string(),
                    to_json_binary(&pool.pool_params)?,
                ),
            ]);

            if let Some(shares) = &pool.total_shares {
                domain_specific_fields
                    .insert("shares_denom".to_string(), to_json_binary(&shares.denom)?);
            }

            for asset in &pool.pool_assets {
                if let Some(token) = &asset.token {
                    domain_specific_fields.insert(
                        format!("pool_asset_{}_weight", token.denom),
                        to_json_binary(&asset.weight)?,
                    );
                }
            }

            let assets = pool
                .pool_assets
                .into_iter()
                .filter_map(|asset| {
                    asset.token.map(|token| {
                        cosmwasm_std::coin(
                            Uint128::from_str(&token.amount).unwrap().into(),
                            token.denom,
                        )
                    })
                })
                .collect();

            let total_shares = pool
                .total_shares
                .map(|shares| shares.amount)
                .unwrap_or_default();

            Ok(ValenceXykPool {
                assets,
                total_shares,
                domain_specific_fields,
            })
        }
    }

    impl TryFrom<ValenceXykPool> for Pool {
        type Error = StdError;

        fn try_from(value: ValenceXykPool) -> Result<Self, Self::Error> {
            // unpack the pool address
            let address: String =
                try_unpack_domain_specific_value("address", &value.domain_specific_fields)?;

            // unpack the pool id
            let id: u64 = try_unpack_domain_specific_value("id", &value.domain_specific_fields)?;

            // unpack the future pool governor
            let future_pool_governor: String = try_unpack_domain_specific_value(
                "future_pool_governor",
                &value.domain_specific_fields,
            )?;

            // unpack the pool params
            let pool_params: Option<PoolParams> =
                try_unpack_domain_specific_value("pool_params", &value.domain_specific_fields)?;

            // unpack the shares denom and total shares amount before combining them to a proto coin
            let shares_denom: String =
                try_unpack_domain_specific_value("shares_denom", &value.domain_specific_fields)?;
            let shares_coin = Coin {
                denom: shares_denom,
                amount: value.total_shares,
            };

            // unpack the total weight
            let total_weight: String =
                try_unpack_domain_specific_value("total_weight", &value.domain_specific_fields)?;

            // unpack the pool assets
            let pool_assets: Vec<PoolAsset> = value
                .assets
                .iter()
                .map(|asset| {
                    let coin = Coin {
                        denom: asset.denom.to_string(),
                        amount: asset.amount.into(),
                    };

                    let weight: String = try_unpack_domain_specific_value(
                        &format!("pool_asset_{}_weight", asset.denom),
                        &value.domain_specific_fields,
                    )
                    .unwrap();

                    PoolAsset {
                        token: Some(coin),
                        weight,
                    }
                })
                .collect();

            Ok(Pool {
                address,
                id,
                pool_params,
                future_pool_governor,
                total_shares: Some(shares_coin),
                pool_assets,
                total_weight,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use cosmwasm_std::{coin, to_json_binary};
        use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
        use std::collections::BTreeMap;

        #[test]
        fn test_try_into() {
            let domain_specific_fields = BTreeMap::from([
                ("address".to_string(), to_json_binary("pool1").unwrap()),
                ("id".to_string(), to_json_binary(&1).unwrap()),
                (
                    "future_pool_governor".to_string(),
                    to_json_binary("gov1").unwrap(),
                ),
                ("total_weight".to_string(), to_json_binary("100").unwrap()),
                (
                    "pool_asset_uatom_weight".to_string(),
                    to_json_binary("120").unwrap(),
                ),
                (
                    "pool_asset_uosmo_weight".to_string(),
                    to_json_binary("80").unwrap(),
                ),
                (
                    "shares_denom".to_string(),
                    to_json_binary("osmo/gamm/whatever").unwrap(),
                ),
                (
                    "pool_params".to_string(),
                    to_json_binary(&Some(PoolParams {
                        swap_fee: "0.003".to_string(),
                        exit_fee: "0.0".to_string(),
                        smooth_weight_change_params: None,
                    }))
                    .unwrap(),
                ),
                ("total_weight".to_string(), to_json_binary("100").unwrap()),
            ]);

            let pool = ValenceXykPool {
                assets: vec![coin(100, "uatom"), coin(200, "uosmo")],
                total_shares: "1000".to_string(),
                domain_specific_fields,
            };

            let osmosis_pool: Pool = pool.try_into().unwrap();

            println!("osmosis_pool: {:?}", osmosis_pool);
            assert_eq!(osmosis_pool.address, "pool1");
            assert_eq!(osmosis_pool.id, 1);
            assert_eq!(osmosis_pool.future_pool_governor, "gov1");
            assert_eq!(osmosis_pool.total_weight, "100");
            assert_eq!(osmosis_pool.pool_assets.len(), 2);
            assert_eq!(osmosis_pool.pool_assets[0].weight, "120");
            assert_eq!(osmosis_pool.pool_assets[1].weight, "80");
            let total_shares = osmosis_pool.total_shares.unwrap();
            assert_eq!(total_shares.amount, "1000");
            assert_eq!(total_shares.denom, "osmo/gamm/whatever");
            assert_eq!(osmosis_pool.pool_params.unwrap().swap_fee, "0.003");
        }

        #[test]
        fn test_try_from() {
            let pool = osmosis_std::types::osmosis::gamm::v1beta1::Pool {
                address: "pool1".to_string(),
                id: 1,
                pool_params: Some(PoolParams {
                    swap_fee: "0.003".to_string(),
                    exit_fee: "0.0".to_string(),
                    smooth_weight_change_params: None,
                }),
                future_pool_governor: "gov1".to_string(),
                total_shares: Some(Coin {
                    denom: "osmo/gamm/whatever".to_string(),
                    amount: "1000".to_string(),
                }),
                pool_assets: vec![
                    PoolAsset {
                        token: Some(Coin {
                            denom: "uatom".to_string(),
                            amount: "100".to_string(),
                        }),
                        weight: "120".to_string(),
                    },
                    PoolAsset {
                        token: Some(Coin {
                            denom: "uosmo".to_string(),
                            amount: "200".to_string(),
                        }),
                        weight: "80".to_string(),
                    },
                ],
                total_weight: "100".to_string(),
            };

            let valence_xyk_pool = ValenceXykPool::try_from(pool).unwrap();

            println!("parsed xyk pool: {:?}", valence_xyk_pool);

            assert_eq!(valence_xyk_pool.assets.len(), 2);
            assert_eq!(valence_xyk_pool.assets[0], coin(100, "uatom"));
            assert_eq!(valence_xyk_pool.assets[1], coin(200, "uosmo"));
            assert_eq!(valence_xyk_pool.total_shares, "1000");
        }
    }
}

pub mod icq {
    use std::collections::BTreeMap;

    use cosmwasm_std::{to_json_binary, Binary, StdError, StdResult};
    use neutron_sdk::bindings::types::{InterchainQueryResult, KVKey};
    use osmosis_std::{shim::Any, types::osmosis::gamm::v1beta1::Pool};

    use crate::middleware::try_unpack_domain_specific_value;
    use prost::Message;

    pub trait IcqIntegration {
        fn get_kv_key(&self, params: BTreeMap<String, Binary>) -> StdResult<KVKey>;
        fn decode_and_reconstruct(
            query_id: String,
            icq_result: InterchainQueryResult,
        ) -> StdResult<Binary>;
    }

    impl IcqIntegration for Pool {
        fn get_kv_key(&self, params: BTreeMap<String, Binary>) -> StdResult<KVKey> {
            let pool_prefix_key: u8 = 0x02;

            let id: u64 = try_unpack_domain_specific_value("pool_id", &params)?;

            let mut pool_access_key = vec![pool_prefix_key];
            pool_access_key.extend_from_slice(&id.to_be_bytes());

            Ok(KVKey {
                path: "gamm".to_string(),
                key: Binary::new(pool_access_key),
            })
        }

        fn decode_and_reconstruct(
            query_id: String,
            icq_result: InterchainQueryResult,
        ) -> StdResult<Binary> {
            let any_msg: Any = Any::decode(icq_result.kv_results[0].value.as_slice())
                .map_err(|e| StdError::generic_err(e.to_string()))?;

            let osmo_pool: Pool = any_msg
                .try_into()
                .map_err(|_| StdError::generic_err("failed to decode pool from any"))?;

            to_json_binary(&osmo_pool)
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

            let kv_key = pool.get_kv_key(params).unwrap();
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

            let osmo_pool_binary = Pool::decode_and_reconstruct(
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
                        denom:
                            "ibc/4E41ED8F3DCAEA15F4D6ADC6EDD7C04A676160735C9710B904B7BF53525B56D6"
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
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, Binary};
    use neutron_sdk::bindings::types::{InterchainQueryResult, StorageValue};
    use osmosis_std::types::osmosis::gamm::v1beta1::Pool;

    use crate::xyk::ValenceXykPool;

    use super::icq::IcqIntegration;

    #[test]
    fn e2e() {
        let b64_key = "AgAAAAAAAAAB";
        let binary_key = Binary::from_base64(b64_key).unwrap();

        let b64_value = "Chovb3Ntb3Npcy5nYW1tLnYxYmV0YTEuUG9vbBKGAgo/b3NtbzE5ZTJtZjdjeXdrdjd6YXVnNm5rNWY4N2QwN2Z4cmRncmxhZHZ5bWgyZ3d2NWNydm0zdm5zdWV3aGg3EAEaBgoBMBIBMCIEMTI4aCokCgtnYW1tL3Bvb2wvMRIVMTAwMDAwMDAwMDAwMDAwMDAwMDAwMl8KUQpEaWJjLzRFNDFFRDhGM0RDQUVBMTVGNEQ2QURDNkVERDdDMDRBNjc2MTYwNzM1Qzk3MTBCOTA0QjdCRjUzNTI1QjU2RDYSCTEwMDAwMDAwMBIKMTA3Mzc0MTgyNDIgChIKBXVvc21vEgkxMDAwMDAwMDASCjEwNzM3NDE4MjQ6CjIxNDc0ODM2NDg=";
        let binary_value = Binary::from_base64(b64_value).unwrap();

        let storage_value = StorageValue {
            storage_prefix: "gamm".to_string(),
            key: binary_key,
            value: binary_value,
        };

        // first we simulate the icq result reconstruction of b64(proto) -> type -> b64(type)
        let osmo_pool_binary = Pool::decode_and_reconstruct(
            Pool::TYPE_URL.to_string(),
            InterchainQueryResult {
                kv_results: vec![storage_value],
                height: 1,
                revision: 1,
            },
        )
        .unwrap();

        // unpack the binary into a type
        let osmo_pool: Pool = from_json(osmo_pool_binary).unwrap();

        // parse the external type into a valence type
        let mut valence_pool: ValenceXykPool = osmo_pool.try_into().unwrap();

        // simulate modifying the pool instance
        valence_pool.assets.push(cosmwasm_std::coin(100, "batom"));
        valence_pool.domain_specific_fields.insert(
            "pool_asset_batom_weight".to_string(),
            cosmwasm_std::to_json_binary(&"120").unwrap(),
        );

        // convert the valence type back into the external type
        let osmo_pool: Pool = valence_pool.try_into().unwrap();

        assert_eq!(osmo_pool.pool_assets.len(), 3);
    }
}
