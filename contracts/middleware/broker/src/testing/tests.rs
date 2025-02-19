use std::collections::BTreeMap;

use cosmwasm_std::{coin, to_json_binary, Binary};
use neutron_sdk::bindings::types::{InterchainQueryResult, StorageValue};
use osmosis_std::types::osmosis::gamm::v1beta1::Pool;
use valence_middleware_utils::{
    canonical_types::pools::xyk::ValenceXykPool, type_registry::types::ValenceType,
};

use super::Suite;

#[test]
fn test_get_kv_key() {
    let mut suite = Suite::default();

    let resp = suite.add_new_registry("1.0.0", suite.registry_addr.to_string());

    println!("resp: {:?}", resp);

    let params = BTreeMap::from([("pool_id".to_string(), to_json_binary(&1u64).unwrap())]);

    let kv_key = suite.get_kv_key(Pool::TYPE_URL, params).unwrap();

    println!("kv_key: {:?}", kv_key);
}

#[test]
fn test_reconstruct_proto() {
    let mut suite = Suite::default();

    suite.add_new_registry("1.0.0", suite.registry_addr.to_string());

    let b64_key = "AgAAAAAAAAAB";
    let binary_key = Binary::from_base64(b64_key).unwrap();

    let b64_value = "Chovb3Ntb3Npcy5nYW1tLnYxYmV0YTEuUG9vbBKGAgo/b3NtbzE5ZTJtZjdjeXdrdjd6YXVnNm5rNWY4N2QwN2Z4cmRncmxhZHZ5bWgyZ3d2NWNydm0zdm5zdWV3aGg3EAEaBgoBMBIBMCIEMTI4aCokCgtnYW1tL3Bvb2wvMRIVMTAwMDAwMDAwMDAwMDAwMDAwMDAwMl8KUQpEaWJjLzRFNDFFRDhGM0RDQUVBMTVGNEQ2QURDNkVERDdDMDRBNjc2MTYwNzM1Qzk3MTBCOTA0QjdCRjUzNTI1QjU2RDYSCTEwMDAwMDAwMBIKMTA3Mzc0MTgyNDIgChIKBXVvc21vEgkxMDAwMDAwMDASCjEwNzM3NDE4MjQ6CjIxNDc0ODM2NDg=";
    let binary_value = Binary::from_base64(b64_value).unwrap();

    let storage_value = StorageValue {
        storage_prefix: "gamm".to_string(),
        key: binary_key,
        value: binary_value,
    };

    let icq_result = InterchainQueryResult {
        kv_results: vec![storage_value],
        height: 1,
        revision: 1,
    };

    let reconstructed_result = suite
        .query_decode_proto(Pool::TYPE_URL, icq_result)
        .unwrap();
    println!("reconstructed result: {:?}", reconstructed_result);
}

#[test]
fn test_into_canonical() {
    let mut suite = Suite::default();

    suite.add_new_registry("1.0.0", suite.registry_addr.to_string());

    let pool = Pool::default();
    println!("pool: {:?}", pool);

    let binary = to_json_binary(&Pool::default()).unwrap();

    let resp: ValenceType = suite.try_to_canonical(Pool::TYPE_URL, binary).unwrap();

    println!("resp: {:?}", resp);
}

#[test]
fn test_from_canonical() {
    let mut suite = Suite::default();

    suite.add_new_registry("1.0.0", suite.registry_addr.to_string());

    let canonical = ValenceType::XykPool(ValenceXykPool {
        assets: vec![
            coin(
                100000000,
                "ibc/4E41ED8F3DCAEA15F4D6ADC6EDD7C04A676160735C9710B904B7BF53525B56D6",
            ),
            coin(100000000, "uosmo"),
        ],
        total_shares: "100000000000000000000".to_string(),
        domain_specific_fields: BTreeMap::from([
            ("pool_asset_ibc/4E41ED8F3DCAEA15F4D6ADC6EDD7C04A676160735C9710B904B7BF53525B56D6_weight".to_string(),
               to_json_binary(&"1073741824").unwrap()
            ),
            ("pool_asset_uosmo_weight".to_string(),
               to_json_binary(&"1073741824").unwrap()
            ),
            (
                "shares_denom".to_string(),
                to_json_binary(&"gamm/pool/1").unwrap(),
            ),
            (
                "address".to_string(),
                to_json_binary(&"osmo19e2mf7cywkv7zaug6nk5f87d07fxrdgrladvymh2gwv5crvm3vnsuewhh7")
                    .unwrap(),
            ),
            ("id".to_string(), to_json_binary(&1).unwrap()),
            (
                "future_pool_governor".to_string(),
                to_json_binary(&"128h").unwrap(),
            ),
            (
                "total_weight".to_string(),
                to_json_binary(&"2147483648").unwrap(),
            ),
            (
                "pool_params".to_string(),
                to_json_binary(&Pool::default().pool_params).unwrap(),
            ),
        ]),
    });

    let native_binary_response = suite.try_from_canonical(canonical).unwrap();

    let revert_canonical: ValenceType = suite
        .try_to_canonical(Pool::TYPE_URL, native_binary_response.binary)
        .unwrap();

    match revert_canonical {
        ValenceType::XykPool(valence_xyk_pool) => {
            assert_eq!(valence_xyk_pool.assets.len(), 2);
            assert_eq!(valence_xyk_pool.total_shares, "100000000000000000000");
            assert_eq!(valence_xyk_pool.domain_specific_fields.len(), 8);
        }
        _ => panic!(),
    }
}
