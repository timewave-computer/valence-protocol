use neutron_test_tube::{
    neutron_std::types::cosmos::bank::v1beta1::QueryAllBalancesRequest, Account, Bank, Module,
};
use valence_astroport_utils::suite::AstroportTestAppBuilder;

#[test]
pub fn test_input_account_balance_initiation() {
    let setup = AstroportTestAppBuilder::new().build().unwrap();

    let bank = Bank::new(&setup.app);

    let balance = bank
        .query_all_balances(&QueryAllBalancesRequest {
            address: setup.input_acc().address(),
            pagination: None,
            resolve_denom: false,
        })
        .unwrap();

    assert_eq!(balance.balances.len(), 2);
    assert!(balance
        .balances
        .iter()
        .any(|token| token.denom == setup.pool_asset1));
    assert!(balance
        .balances
        .iter()
        .any(|token| token.denom == setup.pool_asset2));
}
