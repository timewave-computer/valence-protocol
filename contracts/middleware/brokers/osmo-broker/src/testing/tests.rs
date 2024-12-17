use std::collections::BTreeMap;

use cosmwasm_std::{to_json_binary, Binary};
use neutron_sdk::bindings::types::{InterchainQueryResult, StorageValue};

use super::Suite;

#[test]
fn test_init() {
    let mut suite = Suite::default();

    let resp = suite.add_new_registry("1.0.0", suite.registry_addr.to_string());

    println!("resp: {:?}", resp);

    let params = BTreeMap::from([("pool_id".to_string(), to_json_binary(&1u64).unwrap())]);

    let kv_key = suite.get_kv_key("gamm_pool", params).unwrap();

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

    let reconstructed_result = suite.query_decode_proto("gamm_pool", icq_result).unwrap();
    println!("reconstructed result: {:?}", reconstructed_result);
}
