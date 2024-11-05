use cosmwasm_std::coin;
use valence_osmosis_utils::suite::{OSMO_DENOM, TEST_DENOM};

use super::test_suite::LPerTestSuite;

#[test]
fn test_liquidate_position_basic() {
    LPerTestSuite::new(vec![
        coin(1_000_000u128, OSMO_DENOM),
        coin(1_000_000u128, TEST_DENOM),
    ]);
}
