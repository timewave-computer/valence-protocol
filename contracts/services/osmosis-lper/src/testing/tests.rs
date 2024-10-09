use osmosis_test_tube::{Account, Module, Wasm};
use valence_service_utils::msg::ExecuteMsg;

use crate::{msg::ActionsMsgs, valence_service_integration::OptionalServiceConfig};

use super::test_suite::LPerTestSuite;

#[test]
fn test_init() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);

    let lp_token_bal = setup.query_lp_token_balance(setup.inner.accounts[0].address());
    println!("acc0 lp token bal: {lp_token_bal}");

    wasm.execute::<ExecuteMsg<ActionsMsgs, OptionalServiceConfig>>(
        &setup.lper_addr,
        &ExecuteMsg::ProcessAction(ActionsMsgs::ProvideDoubleSidedLiquidity {}),
        &[],
        setup.inner.processor_acc(),
    )
    .unwrap();
}
