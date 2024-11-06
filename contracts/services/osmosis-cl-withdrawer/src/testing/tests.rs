use cosmwasm_std::coin;
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
    suite.liquidate_position(2, pre_liq_input_acc_position.liquidity);

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
