use crate::msg::QueryTypeDefinition;

use serde_json::{json, Value};

#[test]
fn test_balance_get_kv_key() {
    let balance = osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse::default();
    let params = json!({
        "addr": "osmo1hj5fveer5cjtn4wd6wstzugjfdxzl0xpwhpz63",
        "denom": "uosmo"
    });

    let param_map = params.as_object().unwrap();

    let _result = balance.get_kv_key(param_map.clone()).unwrap();
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
