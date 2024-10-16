use cosmwasm_std::Uint128;
use valence_osmosis_utils::suite::OSMO_DENOM;

use super::test_suite::LPerTestSuite;

#[test]
fn test_provide_liquidity_double_sided() {
    let suite = LPerTestSuite::default();

    let user_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    assert_eq!(user_positions.len(), 0);

    suite.provide_two_sided_liquidity(-1000, 1000);

    let user_positions = suite
        .query_cl_positions(suite.input_acc.to_string())
        .positions;
    assert_eq!(user_positions.len(), 1);
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
