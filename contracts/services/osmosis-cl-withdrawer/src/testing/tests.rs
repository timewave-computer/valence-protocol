use cosmwasm_std::coin;
use valence_osmosis_utils::suite::{OSMO_DENOM, TEST_DENOM};

use super::test_suite::LPerTestSuite;

#[test]
fn test_liquidate_position_basic() {
    let suite = LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ]);

    println!(
        "pre liquidation input acc balances: {:?}",
        suite.inner.query_all_balances(suite.input_acc.as_str())
    );
    println!(
        "pre liquidation output acc balances: {:?}",
        suite.inner.query_all_balances(suite.output_acc.as_str())
    );

    let pre_liq_position = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions[0]
        .position
        .clone()
        .unwrap();

    println!("pre_liq_position: {:?}", pre_liq_position);

    let resp = suite.liquidate_position(2, pre_liq_position.liquidity);
    println!("liquidation response: {:?}", resp.data);

    let post_liq_positions = suite.query_cl_positions(suite.input_acc.to_string());
    println!("post liquidation positions: {:?}", post_liq_positions);

    println!(
        "post liquidation input acc balances: {:?}",
        suite.inner.query_all_balances(suite.input_acc.as_str())
    );
    println!(
        "post liquidation output acc balances: {:?}",
        suite.inner.query_all_balances(suite.output_acc.as_str())
    );
}
