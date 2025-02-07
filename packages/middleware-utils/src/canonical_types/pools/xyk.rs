use std::collections::BTreeMap;

use cosmwasm_schema::{cw_serde, serde::de::DeserializeOwned};
use cosmwasm_std::{ensure, from_json, Binary, Coin, Decimal, StdError, StdResult};

use crate::{
    try_unpack_domain_specific_value,
    type_registry::queries::{ValencePrimitive, ValenceTypeQuery},
};

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

#[cw_serde]
pub enum XykPoolQuery {
    // IMPORTANT: if you add new variants here that return one of the following response types:
    // - String
    // - Uint64
    // - Uint256
    // make sure to extend the unit tests under contracts/middleware/asserter/src/testing
    // to cover that response type assertions.
    GetPrice {},
    GetPoolAssetAmount { target_denom: String },
}

impl ValenceTypeQuery for ValenceXykPool {
    fn query(&self, msg: Binary) -> StdResult<ValencePrimitive> {
        let query_msg: XykPoolQuery = from_json(&msg)?;
        match query_msg {
            XykPoolQuery::GetPrice {} => {
                ensure!(
                    self.assets.len() == 2,
                    StdError::generic_err(
                        "price can be calculated iff xyk pool contains exactly 2 assets"
                    )
                );

                ensure!(
                    !self.assets[0].amount.is_zero() && !self.assets[1].amount.is_zero(),
                    StdError::generic_err(
                        "price can't be calculated if any of the assets amount is zero"
                    )
                );

                let a = self.assets[0].amount;
                let b = self.assets[1].amount;

                let price = Decimal::from_ratio(a, b);
                Ok(ValencePrimitive::Decimal(price))
            }
            XykPoolQuery::GetPoolAssetAmount { target_denom } => {
                // get the coin
                let target_coin = self
                    .assets
                    .iter()
                    .find(|pool_asset| pool_asset.denom == target_denom);

                match target_coin {
                    Some(coin) => Ok(ValencePrimitive::Uint128(coin.amount)),
                    None => Err(StdError::generic_err("target coin not found")),
                }
            }
        }
    }
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
