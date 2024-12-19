use osmosis_std::types::cosmos::bank::v1beta1::QueryBalanceResponse;

pub mod icq_adapter;
pub mod valence_adapter;

const ADDR_KEY: &str = "addr";
const DENOM_KEY: &str = "denom";

pub struct OsmosisBankBalance(pub QueryBalanceResponse);

impl From<QueryBalanceResponse> for OsmosisBankBalance {
    fn from(balance: QueryBalanceResponse) -> Self {
        Self(balance)
    }
}
