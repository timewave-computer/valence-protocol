use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{to_json_binary, Decimal};
use valence_middleware_utils::{
    canonical_types::pools::xyk::{ValenceXykPool, XykPoolQuery},
    type_registry::{queries::ValencePrimitive, types::ValenceType},
};

use crate::{
    msg::{AssertionValue, Predicate, QueryInfo},
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

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::Decimal(
                Decimal::from_str("10.0").unwrap(),
            )),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::Decimal(
                Decimal::from_str("20.0").unwrap(),
            )),
        )
        .is_ok())
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

    assert!(suite
        .assert(
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
            }),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::Decimal(
                Decimal::from_str("20.0").unwrap(),
            )),
        )
        .is_ok())
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

    assert!(suite
        .assert(
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
            }),
            Predicate::LT,
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY_2.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
            }),
        )
        .is_ok())
}

#[test]
fn type_mismatch_assertion() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    assert!(suite
        .assert(
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
            }),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::Decimal(
                Decimal::from_str("20.0").unwrap(),
            )),
        )
        .is_ok())
}

#[test]
fn assertion_string_comparison() {
    let mut suite = Suite::default();

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::String("abc".to_string())),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::String("def".to_string())),
        )
        .is_ok())
}
