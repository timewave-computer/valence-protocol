use cosmwasm_std::coin;

use super::test_suite::LPerTestSuite;

#[test]
fn test_withdraw_liquidity_happy() {
    let lp_token_amt = 50000000000000000000;
    let suite = LPerTestSuite::new(lp_token_amt, None);

    let pre_lp_input_bals = suite.query_all_balances(&suite.input_acc).unwrap();
    let pre_lp_output_bals = suite.query_all_balances(&suite.output_acc).unwrap();

    assert_eq!(
        pre_lp_input_bals,
        vec![coin(
            lp_token_amt,
            suite.inner.pool_cfg.pool_liquidity_token.to_string()
        )]
    );
    assert_eq!(pre_lp_output_bals.len(), 0);

    // withdraw the liquidity
    suite.withdraw_liquidity();

    let post_lp_input_bals = suite.query_all_balances(&suite.input_acc).unwrap();
    let post_lp_output_bals = suite.query_all_balances(&suite.output_acc).unwrap();

    // assert that input account no longer has any tokens,
    // and that the output account now has two tokens
    assert_eq!(post_lp_input_bals.len(), 0);
    assert_eq!(post_lp_output_bals.len(), 2);
}

#[test]
#[should_panic(expected = "input account must have LP tokens to withdraw")]
fn test_withdraw_liquidity_without_lp_tokens() {
    let suite = LPerTestSuite::new(0, None);

    let pre_lp_input_bals = suite.query_all_balances(&suite.input_acc).unwrap();
    assert_eq!(pre_lp_input_bals, vec![]);

    // trying to withdraw liquidity without lp tokens should panic
    suite.withdraw_liquidity();
}
