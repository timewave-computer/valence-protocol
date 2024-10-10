use cosmwasm_std::coin;
use valence_osmosis_utils::suite::OSMO_DENOM;

use super::test_suite::LPerTestSuite;

#[test]
fn test_provide_two_sided_gamm_liquidity() {
    let setup = LPerTestSuite::default();

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 2);
    assert_eq!(output_bals.len(), 0);

    setup.provide_two_sided_liquidity();

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);
}

#[test]
fn test_provide_single_sided_gamm_liquidity() {
    let setup = LPerTestSuite::new(vec![coin(1_000_000u128, OSMO_DENOM)]);

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();

    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);

    setup.provide_single_sided_liquidity();

    let input_bals = setup.query_all_balances(&setup.input_acc).unwrap();
    let output_bals = setup.query_all_balances(&setup.output_acc).unwrap();
    assert_eq!(input_bals.len(), 1);
    assert_eq!(output_bals.len(), 0);
}
