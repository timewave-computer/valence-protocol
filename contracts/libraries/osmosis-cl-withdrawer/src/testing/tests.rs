use std::str::FromStr;

use cosmwasm_std::{coin, Decimal256};
use valence_osmosis_utils::suite::{OSMO_DENOM, TEST_DENOM};

use super::test_suite::LPerTestSuite;

#[test]
fn test_liquidate_position_basic() {
    let suite = LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ]);

    let pre_liq_input_acc_bals = suite
        .inner
        .query_all_balances(suite.input_acc.as_str())
        .unwrap();
    let pre_liq_output_acc_bals = suite
        .inner
        .query_all_balances(suite.output_acc.as_str())
        .unwrap();
    let pre_liq_input_acc_position = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions[0]
        .position
        .clone()
        .unwrap();

    assert_eq!(pre_liq_input_acc_bals, vec![]);
    assert_eq!(pre_liq_output_acc_bals, vec![]);
    assert_eq!(pre_liq_input_acc_position.pool_id, 1);
    assert_eq!(pre_liq_input_acc_position.position_id, 2);

    // liquidate the position
    suite.liquidate_position(2, Some(pre_liq_input_acc_position.liquidity));

    let post_liq_input_acc_bals = suite
        .inner
        .query_all_balances(suite.input_acc.as_str())
        .unwrap();
    let post_liq_output_acc_bals = suite
        .inner
        .query_all_balances(suite.output_acc.as_str())
        .unwrap();
    let post_liq_input_acc_positions = suite.query_cl_positions(suite.input_acc.to_string());

    // assert that there are no more positions and that the output account received the
    // underlying funds
    assert_eq!(post_liq_input_acc_bals, vec![]);
    assert_eq!(post_liq_output_acc_bals.len(), 2);
    assert!(post_liq_input_acc_positions.positions.is_empty());
}

#[test]
#[should_panic(expected = "not the owner of position ID (1)")]
fn test_liquidate_not_owned_position() {
    // position 1 is owned by the admin, not the input acc
    LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ])
    .liquidate_position(1, Some("123".to_string()));
}

#[test]
#[should_panic(expected = "no such position")]
fn test_liquidate_non_existing_position() {
    // position 3 does not exist yet
    LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ])
    .liquidate_position(3, Some("123".to_string()));
}
