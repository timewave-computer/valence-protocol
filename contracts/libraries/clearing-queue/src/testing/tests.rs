use cosmwasm_std::{coin, coins};

use crate::testing::{
    builder::ClearingQueueTestingSuiteBuilder,
    suite::{DENOM_1, DENOM_2},
};

#[test]
fn instantiate_with_valid_cfg() {
    let mut suite = ClearingQueueTestingSuiteBuilder::default()
        .with_input_balances(vec![coin(1_000, DENOM_1), coin(2_000, DENOM_2)])
        .build();

    println!("clearing lib address: {:?}", suite.clearing_queue);

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    println!("input acc d1 bal: {:?}", input_acc_d1_bal);

    let user_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    println!("user1 d1 bal : {:?}", user_d1_bal);

    suite
        .register_new_obligation(suite.user_1.to_string(), coins(500, DENOM_1), 12)
        .unwrap();

    suite.settle_next_obligation().unwrap();

    let input_acc_d1_bal = suite.query_input_acc_bal(DENOM_1);
    println!("input acc d1 bal: {:?}", input_acc_d1_bal);

    let user_d1_bal = suite.query_user_bal(suite.user_1.as_str(), DENOM_1);
    println!("user1 d1 bal : {:?}", user_d1_bal);
}
