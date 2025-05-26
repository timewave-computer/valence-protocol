use cosmwasm_std::{coin, coins};
use valence_library_utils::LibraryAccountType;

use crate::{
    msg::LibraryConfig,
    testing::{
        builder::ClearingQueueTestingSuiteBuilder,
        suite::{DENOM_1, DENOM_2},
    },
};

const INVALID_ADDR: &str = "invalid_addr";

#[test]
#[should_panic(expected = "Error decoding bech32")]
fn test_instantiate_validates_input_acc() {
    ClearingQueueTestingSuiteBuilder::default()
        .with_input_acc(INVALID_ADDR)
        .build();
}

#[test]
#[should_panic(expected = "Error decoding bech32")]
fn test_update_validates_input_acc() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    let new_settlement_acc_addr = LibraryAccountType::Addr(INVALID_ADDR.to_string());

    suite
        .update_clearing_config(LibraryConfig {
            settlement_acc_addr: new_settlement_acc_addr,
        })
        .unwrap();
}

#[test]
#[should_panic(expected = "no pending obligations")]
fn test_settling_obligations_requires_nonempty_queue() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    suite.settle_next_obligation().unwrap();
}

#[test]
#[should_panic(
    expected = "insufficient settlement acc balance to fulfill obligation: 100DENOM_2 < 150DENOM_2"
)]
fn test_settling_obligations_requires_funded_settlement_account() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(vec![coin(100, DENOM_1), coin(100, DENOM_2)])
        .build();

    suite
        .register_new_obligation(
            suite.user_1.to_string(),
            vec![coin(100, DENOM_1), coin(150, DENOM_2)],
            1,
        )
        .unwrap();

    suite.settle_next_obligation().unwrap();
}

#[test]
#[should_panic(expected = "obligation must have payout coins in order to be registered")]
fn test_registering_obligation_validates_payout_coins_len() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    suite
        .register_new_obligation(suite.user_1.to_string(), vec![], 1)
        .unwrap();
}

#[test]
#[should_panic(expected = "Error decoding bech32")]
fn test_registering_obligation_validates_recipient_addr() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    suite
        .register_new_obligation(
            "invalid_addr".to_string(),
            vec![coin(100, DENOM_1), coin(150, DENOM_2)],
            1,
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "obligation payout coin DENOM_1 amount cannot be zero")]
fn test_registering_obligation_validates_payout_coins_nonzero_amounts() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(0, DENOM_1), 1)
        .unwrap();
}

#[test]
fn test_register_withdraw_obligation_happy() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    let queue_len_0 = suite.query_queue_info().len;
    let queue_resp_0 = suite.query_obligations(None, None);

    assert_eq!(queue_len_0, 0);
    assert!(queue_resp_0.obligations.is_empty());

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(100, DENOM_1), 10)
        .unwrap();

    let queue_len = suite.query_queue_info().len;
    let queue_resp = suite.query_obligations(None, None);
    let obligation_status = suite.query_obligation_status(10);

    assert_eq!(queue_len, 1);
    assert_eq!(queue_resp.obligations.len(), 1);
    assert!(!obligation_status.settled);
}

#[test]
fn test_queue_operates_in_fifo_manner() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(vec![coin(1_000, DENOM_1), coin(1_000, DENOM_2)])
        .build();

    let queue_len_0 = suite.query_queue_info().len;
    let queue_resp_0 = suite.query_obligations(None, None);

    assert_eq!(queue_len_0, 0);
    assert!(queue_resp_0.obligations.is_empty());

    //  user_2 -> user_3 -> user_1 -> head
    suite
        .register_new_obligation(suite.user_1.to_string(), coins(100, DENOM_1), 1)
        .unwrap();
    suite
        .register_new_obligation(suite.user_3.to_string(), coins(200, DENOM_2), 2)
        .unwrap();
    suite
        .register_new_obligation(suite.user_2.to_string(), coins(420, DENOM_1), 3)
        .unwrap();

    let queue_len = suite.query_queue_info().len;
    let queue_resp = suite.query_obligations(None, None);

    // first we assert that there is the expected number of obligations in the queue
    assert_eq!(queue_len, 3);
    assert_eq!(queue_resp.obligations.len(), 3);

    // first obligation was user_1
    assert_eq!(queue_resp.obligations[0].recipient, suite.user_1);
    // second obligation was user_3
    assert_eq!(queue_resp.obligations[1].recipient, suite.user_3);
    // third obligation was user_2
    assert_eq!(queue_resp.obligations[2].recipient, suite.user_2);

    // we settle an obligation in order to assert that queue is processed fifo
    suite.settle_next_obligation().unwrap();

    let queue_len = suite.query_queue_info().len;
    let queue_resp = suite.query_obligations(None, None);

    // first we assert that there is one less obligation in the queue
    assert_eq!(queue_len, 2);
    assert_eq!(queue_resp.obligations.len(), 2);

    // user_3 should be the oldest
    assert_eq!(queue_resp.obligations[0].recipient, suite.user_3);
    // user_2 should be the latest
    assert_eq!(queue_resp.obligations[1].recipient, suite.user_2);
}

#[test]
#[should_panic(expected = "obligation #10 is already registered in the queue")]
fn test_double_accounting_errors() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default().build();

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(100, DENOM_1), 10)
        .unwrap();
    suite
        .register_new_obligation(suite.user_1.to_string(), coins(200, DENOM_1), 10)
        .unwrap();
}

#[test]
#[should_panic(expected = "obligation #10 is already registered in the queue")]
fn test_double_accounting_errors_after_obligation_settlement() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(vec![coin(1_000, DENOM_1), coin(1_000, DENOM_2)])
        .build();

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(100, DENOM_1), 10)
        .unwrap();

    suite.settle_next_obligation().unwrap();

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(200, DENOM_1), 10)
        .unwrap();
}

#[test]
fn test_user_obligation_with_multiple_denoms() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(vec![coin(100, DENOM_1), coin(100, DENOM_2)])
        .build();

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    let input_acc_d2_bal = suite.query_input_acc_bal(DENOM_2);
    let u1_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    let u1_d2_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_2);

    assert_eq!(input_acc_d1_bal.amount.u128(), 100);
    assert_eq!(input_acc_d2_bal.amount.u128(), 100);
    assert!(u1_d1_bal.amount.is_zero());
    assert!(u1_d2_bal.amount.is_zero());

    suite
        .register_new_obligation(
            suite.user_1.to_string(),
            vec![coin(100, DENOM_1), coin(100, DENOM_2)],
            1,
        )
        .unwrap();

    let obligation_status = suite.query_obligation_status(1);
    assert!(!obligation_status.settled);

    suite.settle_next_obligation().unwrap();

    let obligation_status = suite.query_obligation_status(1);
    assert!(obligation_status.settled);

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    let input_acc_d2_bal = suite.query_input_acc_bal(DENOM_2);
    let u1_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    let u1_d2_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_2);

    assert!(input_acc_d1_bal.amount.is_zero());
    assert!(input_acc_d2_bal.amount.is_zero());

    assert_eq!(u1_d1_bal.amount.u128(), 100);
    assert_eq!(u1_d2_bal.amount.u128(), 100);
}

#[test]
fn test_multi_user_settlement() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(coins(1_000, DENOM_1))
        .build();

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(100, DENOM_1), 1)
        .unwrap();
    suite
        .register_new_obligation(suite.user_2.to_string(), coins(300, DENOM_1), 2)
        .unwrap();
    suite
        .register_new_obligation(suite.user_3.to_string(), coins(400, DENOM_1), 3)
        .unwrap();

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    let u1_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    let u2_d1_bal = suite.query_user_bal(suite.user_2.as_str(), DENOM_1);
    let u3_d1_bal = suite.query_user_bal(suite.user_3.as_str(), DENOM_1);

    assert_eq!(input_acc_d1_bal.amount.u128(), 1_000);
    assert!(u1_d1_bal.amount.is_zero());
    assert!(u2_d1_bal.amount.is_zero());
    assert!(u3_d1_bal.amount.is_zero());

    assert!(!suite.query_obligation_status(1).settled);
    assert!(!suite.query_obligation_status(2).settled);
    assert!(!suite.query_obligation_status(3).settled);

    // settle all existing obligations
    suite.settle_next_obligation().unwrap();
    suite.settle_next_obligation().unwrap();
    suite.settle_next_obligation().unwrap();

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    let u1_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    let u2_d1_bal = suite.query_user_bal(suite.user_2.as_str(), DENOM_1);
    let u3_d1_bal = suite.query_user_bal(suite.user_3.as_str(), DENOM_1);

    // assert that the obligations are now considered as settled
    assert!(suite.query_obligation_status(1).settled);
    assert!(suite.query_obligation_status(2).settled);
    assert!(suite.query_obligation_status(3).settled);

    // assert that the settlement account balance is decreased as expected
    assert_eq!(input_acc_d1_bal.amount.u128(), 1_000 - 100 - 300 - 400);
    // assert that users were credited as expected
    assert_eq!(u1_d1_bal.amount.u128(), 100);
    assert_eq!(u2_d1_bal.amount.u128(), 300);
    assert_eq!(u3_d1_bal.amount.u128(), 400);
}

#[test]
fn test_multi_denom_settlement() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(vec![coin(1_000, DENOM_1), coin(1_000, DENOM_2)])
        .build();

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(100, DENOM_1), 1)
        .unwrap();
    suite
        .register_new_obligation(suite.user_1.to_string(), coins(300, DENOM_2), 2)
        .unwrap();

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    let input_acc_d2_bal = suite.query_input_acc_bal(DENOM_2);
    let u1_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    let u1_d2_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_2);

    assert_eq!(input_acc_d1_bal.amount.u128(), 1_000);
    assert_eq!(input_acc_d2_bal.amount.u128(), 1_000);
    assert!(u1_d1_bal.amount.is_zero());
    assert!(u1_d2_bal.amount.is_zero());

    // settle all existing obligations
    suite.settle_next_obligation().unwrap();
    suite.settle_next_obligation().unwrap();

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    let input_acc_d2_bal = suite.query_input_acc_bal(DENOM_2);
    let u1_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    let u1_d2_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_2);

    assert_eq!(input_acc_d1_bal.amount.u128(), 1_000 - 100);
    assert_eq!(input_acc_d2_bal.amount.u128(), 1_000 - 300);

    // assert that user was credited as expected
    assert_eq!(u1_d1_bal.amount.u128(), 100);
    assert_eq!(u1_d2_bal.amount.u128(), 300);
}
