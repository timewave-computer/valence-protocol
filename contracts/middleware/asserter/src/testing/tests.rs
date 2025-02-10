use std::{collections::BTreeMap, str::FromStr};

use cosmwasm_std::{to_json_binary, Decimal, Uint128, Uint256, Uint64};
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
fn double_const_decimal_assertion_happy() {
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
fn var_const_decimal_assertion_happy() {
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
#[should_panic(expected = "assertion failed")]
fn var_const_decimal_assertion_err() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    suite
        .assert(
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
            }),
            Predicate::GT,
            AssertionValue::Constant(ValencePrimitive::Decimal(
                Decimal::from_str("20.0").unwrap(),
            )),
        )
        .unwrap();
}

#[test]
fn var_var_decimal_assertion_happy() {
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
#[should_panic(expected = "variant mismatch")]
fn type_mismatch_assertion_err() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    suite
        .assert(
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPrice {}).unwrap(),
            }),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::String("20.0".to_string())),
        )
        .unwrap();
}

#[test]
fn double_const_string_assertion_lt_happy() {
    let mut suite = Suite::default();

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::String("abc".to_string())),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::String("def".to_string())),
        )
        .is_ok())
}

#[test]
fn double_const_string_assertion_eq_happy() {
    let mut suite = Suite::default();

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::String("abc".to_string())),
            Predicate::EQ,
            AssertionValue::Constant(ValencePrimitive::String("abc".to_string())),
        )
        .is_ok())
}

#[test]
#[should_panic(expected = "assertion failed")]
fn double_const_string_assertion_err() {
    let mut suite = Suite::default();

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::String("a".to_string())),
            Predicate::GT,
            AssertionValue::Constant(ValencePrimitive::String("def".to_string())),
        )
        .is_ok())
}

#[test]
#[should_panic(expected = "assertion failed")]
fn var_const_u128_assertion_err() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    suite
        .assert(
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPoolAssetAmount {
                    target_denom: "untrn".to_string(),
                })
                .unwrap(),
            }),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::Uint128(Uint128::new(100_000))),
        )
        .unwrap();
}

#[test]
fn var_var_u128_assertion_happy() {
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
                query: to_json_binary(&XykPoolQuery::GetPoolAssetAmount {
                    target_denom: "untrn".to_string(),
                })
                .unwrap(),
            }),
            Predicate::LT,
            AssertionValue::Variable(QueryInfo {
                storage_account: suite.storage_account.to_string(),
                storage_slot_key: STORAGE_SLOT_KEY_2.to_string(),
                query: to_json_binary(&XykPoolQuery::GetPoolAssetAmount {
                    target_denom: "untrn".to_string(),
                })
                .unwrap(),
            }),
        )
        .is_ok())
}

#[test]
fn double_const_u256_assertion_lte_happy() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::Uint256(Uint256::zero())),
            Predicate::LTE,
            AssertionValue::Constant(ValencePrimitive::Uint256(Uint256::one())),
        )
        .is_ok());

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::Uint256(Uint256::zero())),
            Predicate::LT,
            AssertionValue::Constant(ValencePrimitive::Uint256(Uint256::one())),
        )
        .is_ok());
}

#[test]
fn double_const_u64_assertion_gte_happy() {
    let mut suite = Suite::default();

    let xyk_pool = ValenceXykPool {
        assets: Suite::default_coins(),
        total_shares: "10".to_string(),
        domain_specific_fields: BTreeMap::new(),
    };
    suite.post_valence_type(STORAGE_SLOT_KEY, ValenceType::XykPool(xyk_pool));

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::Uint64(Uint64::new(3145))),
            Predicate::GTE,
            AssertionValue::Constant(ValencePrimitive::Uint64(Uint64::new(145))),
        )
        .is_ok());

    assert!(suite
        .assert(
            AssertionValue::Constant(ValencePrimitive::Uint64(Uint64::new(3145))),
            Predicate::GT,
            AssertionValue::Constant(ValencePrimitive::Uint64(Uint64::new(145))),
        )
        .is_ok())
}
