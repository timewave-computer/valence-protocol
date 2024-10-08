use osmosis_test_tube::{Module, Wasm};

use crate::test_suite::LPerTestSuite;

#[test]
fn test_init() {
    let setup = LPerTestSuite::default();
    let wasm = Wasm::new(&setup.inner.app);
}
