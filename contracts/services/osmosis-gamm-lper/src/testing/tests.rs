use std::str::FromStr;

use cosmwasm_std::{coin, Decimal};
use valence_osmosis_utils::{
    suite::OSMO_DENOM,
    utils::{DecimalRange, LiquidityProviderConfig},
};

use super::test_suite::LPerTestSuite;

#[test]
#[should_panic(expected = "Pool does not contain expected assets")]
fn test_provide_liquidity_fails_validation() {
    LPerTestSuite::new(
        vec![coin(1_000_000u128, OSMO_DENOM)],
        Some(LiquidityProviderConfig {
            pool_id: 1,
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: "random_denom".to_string(),
        }),
    );
}

#[test]
#[should_panic(expected = "Value is not within the expected range")]
fn test_provide_two_sided_liquidity_out_of_range() {
    let setup = LPerTestSuite::default();

    setup.provide_two_sided_liquidity(Some(DecimalRange::from((
        Decimal::from_str("0.0009").unwrap(),
        Decimal::from_str("0.1111").unwrap(),
    ))));
}

#[test]
fn test_provide_two_sided_liquidity_no_range() {
    let setup = LPerTestSuite::default();

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 2);
    assert_eq!(output_bals.len(), 0);

    setup.provide_two_sided_liquidity(None);

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 0);
    assert_eq!(output_bals.len(), 1);
}

#[test]
fn test_provide_two_sided_liquidity_valid_range() {
    let setup = LPerTestSuite::default();

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 2);
    assert_eq!(output_bals.len(), 0);

    setup.provide_two_sided_liquidity(Some(DecimalRange::from((
        Decimal::from_str("0.9").unwrap(),
        Decimal::from_str("1.1").unwrap(),
    ))));

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 0);
    assert_eq!(output_bals.len(), 1);
}

#[test]
#[should_panic(expected = "Value is not within the expected range")]
fn test_provide_single_sided_liquidity_out_of_range() {
    let setup = LPerTestSuite::default();

    setup.provide_single_sided_liquidity(
        OSMO_DENOM,
        10_000u128.into(),
        Some(DecimalRange::from((
            Decimal::from_str("0.00001").unwrap(),
            Decimal::from_str("0.11111").unwrap(),
        ))),
    );
}

#[test]
fn test_provide_single_sided_liquidity_no_range() {
    let setup = LPerTestSuite::new(vec![coin(1_000_000u128, OSMO_DENOM)], None);

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);

    setup.provide_single_sided_liquidity(OSMO_DENOM, 10_000u128.into(), None);

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 1);
}

#[test]
fn test_provide_single_sided_liquidity_valid_range() {
    let setup = LPerTestSuite::new(vec![coin(1_000_000u128, OSMO_DENOM)], None);

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);

    setup.provide_single_sided_liquidity(
        OSMO_DENOM,
        10_000u128.into(),
        Some(DecimalRange::from((
            Decimal::from_str("0.9").unwrap(),
            Decimal::from_str("1.1").unwrap(),
        ))),
    );

    let input_bals = setup.inner.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.inner.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 1);
}
