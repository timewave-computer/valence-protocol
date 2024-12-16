use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

use crate::MiddlewareError;

pub trait ValenceBankBalanceAdapter {
    type External;

    fn try_to_canonical(&self) -> Result<ValenceBankBalance, MiddlewareError>;
    fn try_from_canonical(canonical: ValenceBankBalance)
        -> Result<Self::External, MiddlewareError>;
}

#[cw_serde]
pub struct ValenceBankBalance {
    pub assets: Vec<Coin>,
}
