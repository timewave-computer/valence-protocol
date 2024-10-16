use cosmwasm_std::Uint128;
use valence_osmosis_utils::suite::OSMO_DENOM;

use super::test_suite::LPerTestSuite;

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
    suite.provide_two_sided_liquidity(-1000, 1000, 0, 1_000_000);

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
    let suite = LPerTestSuite::default();

    let user_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    assert_eq!(user_positions.len(), 0);

    suite.provide_single_sided_liquidity(OSMO_DENOM, Uint128::new(1000), -1000, 1000);

    let user_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    assert_eq!(user_positions.len(), 1);
}
