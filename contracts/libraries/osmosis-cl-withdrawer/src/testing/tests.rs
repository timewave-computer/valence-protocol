use std::str::FromStr;

use cosmwasm_std::{coin, Decimal256};
use valence_osmosis_utils::suite::{OSMO_DENOM, TEST_DENOM};

use super::test_suite::LPerTestSuite;

#[test]
fn test_liquidate_position_with_amount_specified() {
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

    let available_liquidity = Decimal256::from_str(&pre_liq_input_acc_position.liquidity).unwrap();

    // calculate 1/2 of the available position liquidity
    let half_of_liquidity = available_liquidity / Decimal256::from_str("2.0").unwrap().atomics();

    // liquidate the half of the position
    suite.liquidate_position(2, Some(half_of_liquidity.to_string()));

    let post_liq_input_acc_bals = suite
        .inner
        .query_all_balances(suite.input_acc.as_str())
        .unwrap();
    let post_liq_output_acc_bals = suite
        .inner
        .query_all_balances(suite.output_acc.as_str())
        .unwrap();
    let post_liq_input_acc_positions = suite.query_cl_positions(suite.input_acc.to_string());

    // assert that the position still exists and that the output account received 1/2
    // of the underlying funds
    assert_eq!(post_liq_input_acc_bals, vec![]);
    assert_eq!(post_liq_output_acc_bals.len(), 2);
    assert_eq!(post_liq_input_acc_positions.positions.len(), 1);

    // liquidate the remaining position
    suite.liquidate_position(2, Some(half_of_liquidity.to_string()));

    let final_liq_input_acc_positions = suite.query_cl_positions(suite.input_acc.to_string());

    // assert that the position no longer exists
    assert!(final_liq_input_acc_positions.positions.is_empty());
}

#[test]
fn test_liquidate_position_default_amount() {
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

    // liquidate the entire position by not specifying the amount
    // which defaults to the entire position
    suite.liquidate_position(2, None);

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
#[should_panic]
fn test_liquidate_non_existing_position() {
    // position 3 does not exist yet
    LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ])
    .liquidate_position(3, Some("123".to_string()));
}

#[test]
#[should_panic(expected = "insufficient liquidity:")]
fn test_liquidate_insufficient_liquidity_amount() {
    let suite = LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ]);

    let pre_liq_input_acc_position = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions[0]
        .position
        .clone()
        .unwrap();

    let available_liquidity = Decimal256::from_str(&pre_liq_input_acc_position.liquidity).unwrap();

    // calculate 2x of the available position liquidity
    let double_liquidity = available_liquidity
        .checked_add(available_liquidity)
        .unwrap()
        .atomics();

    // attempt to liquidate 2x the available liquidity
    suite.liquidate_position(2, Some(double_liquidity.to_string()));
}
