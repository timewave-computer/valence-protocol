use super::test_suite::LPerTestSuite;

#[test]
fn test_provide_liquidity_base() {
    let suite = LPerTestSuite::new(None);

    let bals = suite.query_all_balances(&suite.input_acc).unwrap();

    println!("bals: {:?}", bals);
}
