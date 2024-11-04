use super::test_suite::LPerTestSuite;

#[test]
fn test_provide_liquidity_base() {
    let suite = LPerTestSuite::new(None);

    let bals = suite.query_all_balances(&suite.input_acc).unwrap();
    println!("input bals: {:?}", bals);

    let bals = suite.query_all_balances(&suite.output_acc).unwrap();
    println!("output bals: {:?}", bals);

    let resp = suite.withdraw_liquidity();
    println!("resp: {:?}", resp);

    let bals = suite.query_all_balances(&suite.input_acc).unwrap();
    println!("input bals: {:?}", bals);

    let bals = suite.query_all_balances(&suite.output_acc).unwrap();
    println!("output bals: {:?}", bals);
}
