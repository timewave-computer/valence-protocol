use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, serde::de::DeserializeOwned};
use cosmwasm_std::{ensure, Binary, Coin, Decimal, StdError, StdResult};

use crate::try_unpack_domain_specific_value;

#[cw_serde]
pub struct ValenceXykPool {
    /// assets in the pool
    pub assets: Vec<Coin>,

    /// total amount of shares issued
    pub total_shares: String,

    /// any other fields that are unique to the external pool type
    /// being represented by this struct
    pub domain_specific_fields: BTreeMap<String, Binary>,
}

impl ValenceXykPool {
    pub fn get_price(&self) -> StdResult<Decimal> {
        ensure!(
            self.assets.len() == 2,
            StdError::generic_err("price can be calculated iff xyk pool contains exactly 2 assets")
        );

        ensure!(
            !self.assets[0].amount.is_zero() && !self.assets[1].amount.is_zero(),
            StdError::generic_err("price can't be calculated if any of the assets amount is zero")
        );

        let a = self.assets[0].amount;
        let b = self.assets[1].amount;

        Ok(Decimal::from_ratio(a, b))
    }

    pub fn get_domain_specific_field<T>(&self, key: &str) -> StdResult<T>
    where
        T: DeserializeOwned,
    {
        try_unpack_domain_specific_value(key, &self.domain_specific_fields)
    }
}
