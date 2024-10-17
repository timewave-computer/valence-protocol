use cosmwasm_std::{coin, coins, Uint64};
use valence_osmosis_utils::suite::OSMO_DENOM;

use crate::msg::LiquidityProviderConfig;

use super::test_suite::LPerTestSuite;

#[test]
#[should_panic(expected = "Pool does not contain expected assets")]
fn test_provide_liquidity_fails_validation() {
    LPerTestSuite::new(
        vec![coin(1_000_000u128, OSMO_DENOM)],
        Some(LiquidityProviderConfig {
            pool_id: Uint64::one(),
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: "random_denom".to_string(),
        }),
    );
}

#[test]
fn test_provide_liquidity_double_sided() {
    let suite = LPerTestSuite::default();

    let input_acc_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    let output_acc_positions = suite
        .query_cl_positions(suite.output_acc.to_string())
        .positions;
    assert_eq!(input_acc_positions.len(), 0);
    assert_eq!(output_acc_positions.len(), 0);
    let input_balances = suite.inner.query_all_balances(suite.input_acc.as_str());
    println!("input balances pre-lp: {:?}", input_balances);

    suite.provide_two_sided_liquidity(-1000, 0, 0, 0);

    let input_acc_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    let output_acc_positions = suite
        .query_cl_positions(suite.output_acc.to_string())
        .positions;
    assert_eq!(input_acc_positions.len(), 0);
    assert_eq!(output_acc_positions.len(), 1);
    let input_balances = suite.inner.query_all_balances(suite.input_acc.as_str());
    println!("input balances post-lp: {:?}", input_balances);
}

#[test]
fn test_provide_liquidity_single_sided() {
    let suite = LPerTestSuite::new(coins(1_000_000, OSMO_DENOM), None);

    let input_acc_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    let output_acc_positions = suite
        .query_cl_positions(suite.output_acc.to_string())
        .positions;
    assert_eq!(input_acc_positions.len(), 0);
    assert_eq!(output_acc_positions.len(), 0);
    let input_balances = suite.inner.query_all_balances(suite.input_acc.as_str());
    println!("input balances pre-lp: {:?}", input_balances);

    suite.provide_single_sided_liquidity(OSMO_DENOM, 1000000, 0, 1000);

    let input_acc_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    let output_acc_positions = suite
        .query_cl_positions(suite.output_acc.to_string())
        .positions;
    assert_eq!(input_acc_positions.len(), 0);
    assert_eq!(output_acc_positions.len(), 1);
    let input_balances = suite.inner.query_all_balances(suite.input_acc.as_str());
    println!("input balances post-lp: {:?}", input_balances);
}
