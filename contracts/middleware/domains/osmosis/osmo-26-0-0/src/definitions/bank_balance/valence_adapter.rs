use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;
use valence_middleware_utils::{
    canonical_types::bank::balance::{ValenceBankBalance, ValenceBankBalanceAdapter},
    MiddlewareError,
};

use super::OsmosisBankBalance;

impl ValenceBankBalanceAdapter for OsmosisBankBalance {
    type External = QueryBalanceResponse;

    fn try_to_canonical(&self) -> Result<ValenceBankBalance, MiddlewareError> {
        todo!()
    }

    fn try_from_canonical(
        canonical: ValenceBankBalance,
    ) -> Result<Self::External, MiddlewareError> {
        todo!()
    }
}
