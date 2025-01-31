use std::collections::BTreeMap;

use cosmwasm_std::to_json_binary;
use valence_middleware_utils::{
    canonical_types::pools::xyk::{ValenceXykPool, XykPoolQuery},
    type_registry::types::ValenceType,
};

use crate::{
    msg::{AssertionConfig, AssertionValue, Predicate, QueryInfo, ValueType},
    testing::{Suite, STORAGE_SLOT_KEY},
};

use super::STORAGE_SLOT_KEY_2;

#[test]
fn base_const_assertion() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    let assertion_cfg = AssertionConfig {
        a: AssertionValue::Constant(to_json_binary("10.0").unwrap()),
        predicate: Predicate::LT,
        b: AssertionValue::Constant(to_json_binary("20.0").unwrap()),
        ty: ValueType::Decimal,
    };

    match suite.query_assert(assertion_cfg) {
        Ok(_) => {
            println!("assertion passed");
        }
        Err(e) => panic!("failed: {e}"),
    }
}

#[test]
fn base_variable_assertion() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    let assertion_cfg = AssertionConfig {
        a: AssertionValue::Variable(QueryInfo {
            storage_account: suite.storage_account.to_string(),
            storage_slot_key: STORAGE_SLOT_KEY.to_string(),
            query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
        }),
        predicate: Predicate::LT,
        b: AssertionValue::Constant(to_json_binary("20.0").unwrap()),
        ty: ValueType::Decimal,
    };

    match suite.query_assert(assertion_cfg) {
        Ok(_) => {
            println!("assertion passed");
        }
        Err(e) => panic!("failed: {e}"),
    }
}

#[test]
fn double_variable_assertion() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    let xyk_pool_2 = ValenceXykPool {
        assets: Suite::default_coins_2(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY_2, ValenceType::XykPool(xyk_pool_2));

    let assertion_cfg = AssertionConfig {
        a: AssertionValue::Variable(QueryInfo {
            storage_account: suite.storage_account.to_string(),
            storage_slot_key: STORAGE_SLOT_KEY.to_string(),
            query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
        }),
        predicate: Predicate::LT,
        b: AssertionValue::Variable(QueryInfo {
            storage_account: suite.storage_account.to_string(),
            storage_slot_key: STORAGE_SLOT_KEY_2.to_string(),
            query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
        }),
        ty: ValueType::Decimal,
    };

    match suite.query_assert(assertion_cfg) {
        Ok(_) => {
            println!("assertion passed");
        }
        Err(e) => panic!("failed: {e}"),
    }
}
