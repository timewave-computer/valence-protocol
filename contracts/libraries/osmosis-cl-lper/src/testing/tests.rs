use cosmwasm_std::{coin, Int64, Uint64};
use valence_osmosis_utils::{
    suite::{OSMO_DENOM, TEST_DENOM},
    utils::cl_utils::TickRange,
};

use crate::msg::LiquidityProviderConfig;

use super::test_suite::LPerTestSuite;

#[test]
#[should_panic]
fn test_provide_liquidity_fails_validation_pool_not_found() {
    LPerTestSuite::new(
        vec![
            coin(1_000_000u128, OSMO_DENOM),
            coin(1_000_000u128, TEST_DENOM),
        ],
        Some(LiquidityProviderConfig {
            pool_id: Uint64::new(3),
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: TEST_DENOM.to_string(),
            global_tick_range: TickRange {
                lower_tick: Int64::MIN,
                upper_tick: Int64::MAX,
            },
        }),
    );
}

#[test]
#[should_panic(expected = "Pool does not contain expected assets")]
fn test_provide_liquidity_fails_validation_denom_mismatch() {
    LPerTestSuite::new(
        vec![
            coin(1_000_000u128, OSMO_DENOM),
            coin(1_000_000u128, TEST_DENOM),
        ],
        Some(LiquidityProviderConfig {
            pool_id: Uint64::one(),
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: "random_denom".to_string(),
            global_tick_range: TickRange {
                lower_tick: Int64::MIN,
                upper_tick: Int64::MAX,
            },
        }),
    );
}

#[test]
fn test_provide_liquidity_custom() {
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
    println!("input balances pre-lp: {input_balances:?}");

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("PRE-LP pool: {pool:?}");
    suite.provide_liquidity_custom(-1000, 0, 0, 0);

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("POST-LP pool: {pool:?}");
    let input_acc_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    let output_acc_positions = suite
        .query_cl_positions(suite.output_acc.to_string())
        .positions;
    assert_eq!(input_acc_positions.len(), 0);
    assert_eq!(output_acc_positions.len(), 1);
    let input_balances = suite.inner.query_all_balances(suite.input_acc.as_str());
    println!("input balances post-lp: {input_balances:?}");
}

#[test]
fn test_provide_liquidity_default() {
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
    println!("input balances pre-lp: {input_balances:?}");

    let pool = suite.query_cl_pool(suite.inner.pool_cfg.pool_id.u64());
    println!("PRE-LP pool current tick: {:?}", pool.current_tick);

    suite.provide_liquidity_default(1050);

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
    println!("input balances post-lp: {input_balances:?}");
}

#[test]
#[should_panic(expected = "tick range is not a multiple of the other")]
fn test_provide_liquidity_custom_with_disrespectful_range() {
    let suite = LPerTestSuite::default();

    // pool's tick spacing is 100, this range should fail
    suite.provide_liquidity_custom(-150, 250, 0, 0);
}

#[test]
#[should_panic(expected = "lower tick must be less than upper tick")]
fn test_provide_liquidity_custom_invalid_tick_range() {
    let suite = LPerTestSuite::default();

    suite.provide_liquidity_custom(1000, -1000, 0, 0);
}

#[test]
#[should_panic(expected = "other tick range is not contained by this range")]
fn test_provide_liquidity_default_validates_global_min_max_range() {
    let suite = LPerTestSuite::new(
        vec![
            coin(1_000_000u128, OSMO_DENOM),
            coin(1_000_000u128, TEST_DENOM),
        ],
        Some(LiquidityProviderConfig {
            pool_id: Uint64::one(),
            pool_asset_1: OSMO_DENOM.to_string(),
            pool_asset_2: TEST_DENOM.to_string(),
            global_tick_range: TickRange {
                lower_tick: Int64::new(-100),
                upper_tick: Int64::new(100),
            },
        }),
    );

    // This should fail because the resulting range will exceed the global tick range
    suite.provide_liquidity_default(1000);
}
