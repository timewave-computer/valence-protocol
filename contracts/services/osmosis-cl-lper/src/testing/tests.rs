use cosmwasm_std::{coin, coins, Uint64};
use valence_osmosis_utils::suite::{OSMO_DENOM, TEST_DENOM};

use crate::msg::LiquidityProviderConfig;

use super::test_suite::LPerTestSuite;

#[test]
#[should_panic]
fn test_provide_liquidity_fails_validation_pool_not_found() {
    LPerTestSuite::new(
        vec![coin(1_000_000u128, OSMO_DENOM)],
        Some(LiquidityProviderConfig {
            pool_id: Uint64::new(3),
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: TEST_DENOM.to_string(),
        }),
    );
}

#[test]
#[should_panic(expected = "Pool does not contain expected assets")]
fn test_provide_liquidity_fails_validation_denom_mismatch() {
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

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("PRE-LP pool: {:?}", pool);
    suite.provide_two_sided_liquidity(-1000, 0, 0, 0);

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("POST-LP pool: {:?}", pool);
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
#[should_panic(expected = "current tick 0 not in range (-1001, -1)")]
fn test_provide_liquidity_double_sided_validates_tick_range() {
    let suite = LPerTestSuite::default();

    suite.provide_two_sided_liquidity(-1001, -1, 0, 0);
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

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("PRE-LP pool current tick: {:?}", pool.current_tick);

    suite.provide_single_sided_liquidity(OSMO_DENOM, 1000000, 0, 1000);

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("POST-LP pool current tick: {:?}", pool.current_tick);

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
#[should_panic(expected = "current tick 0 not in range (-1001, -1)")]
fn test_provide_liquidity_single_sided_validates_tick_range() {
    let suite = LPerTestSuite::default();

    suite.provide_two_sided_liquidity(-1001, -1, 0, 0);
}
