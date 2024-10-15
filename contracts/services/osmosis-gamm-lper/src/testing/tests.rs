use cosmwasm_std::coin;
use valence_osmosis_utils::suite::OSMO_DENOM;

use crate::msg::DecimalRange;

use super::test_suite::LPerTestSuite;

#[test]
#[should_panic(expected = "Value is not within the expected range")]
fn test_provide_two_sided_liquidity_out_of_range() {
    let setup = LPerTestSuite::default();

    setup.provide_two_sided_liquidity(Some(DecimalRange::from_strs("0.00001", "0.00002").unwrap()));
}

#[test]
fn test_provide_two_sided_liquidity_no_range() {
    let setup = LPerTestSuite::default();

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 2);
    assert_eq!(output_bals.len(), 0);

    setup.provide_two_sided_liquidity(None);

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 0);
    assert_eq!(output_bals.len(), 1);
}

#[test]
fn test_provide_two_sided_liquidity_valid_range() {
    let setup = LPerTestSuite::default();

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 2);
    assert_eq!(output_bals.len(), 0);

    setup.provide_two_sided_liquidity(Some(DecimalRange::from_strs("0.9", "1.1").unwrap()));

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();
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
        Some(DecimalRange::from_strs("0.00001", "0.00002").unwrap()),
    );
}

#[test]
fn test_provide_single_sided_liquidity_no_range() {
    let setup = LPerTestSuite::new(vec![coin(1_000_000u128, OSMO_DENOM)]);

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);

    setup.provide_single_sided_liquidity(OSMO_DENOM, 10_000u128.into(), None);

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 1);
}

#[test]
fn test_provide_single_sided_liquidity_valid_range() {
    let setup = LPerTestSuite::new(vec![coin(1_000_000u128, OSMO_DENOM)]);

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);

    setup.provide_single_sided_liquidity(
        OSMO_DENOM,
        10_000u128.into(),
        Some(DecimalRange::from_strs("0.9", "1.1").unwrap()),
    );

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 1);
}
