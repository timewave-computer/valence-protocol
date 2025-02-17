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
    // TODO: move this into the ValenceTypeQuery and use generics
    pub fn get_domain_specific_field<T>(&self, key: &str) -> StdResult<T>
    where
        T: DeserializeOwned,
    {
        try_unpack_domain_specific_value(key, &self.domain_specific_fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coin, to_json_binary, Uint128};

    #[test]
    fn test_get_price_happy() {
        let pool = ValenceXykPool {
            assets: vec![coin(100, "token_a"), coin(200, "token_b")],
            total_shares: "1000".to_string(),
            domain_specific_fields: BTreeMap::new(),
        };

        let query_msg = XykPoolQuery::GetPrice {};
        let result = pool.query(to_json_binary(&query_msg).unwrap()).unwrap();
        assert_eq!(
            result,
            ValencePrimitive::Decimal(Decimal::from_ratio(100u128, 200u128))
        );
    }

    #[test]
    #[should_panic(expected = "price can't be calculated if any of the assets amount is zero")]
    fn test_get_price_asset_0_zero_amount_err() {
        let pool = ValenceXykPool {
            assets: vec![coin(0, "token_a"), coin(200, "token_b")],
            total_shares: "1000".to_string(),
            domain_specific_fields: BTreeMap::new(),
        };
        let query_msg = XykPoolQuery::GetPrice {};

        pool.query(to_json_binary(&query_msg).unwrap()).unwrap();
    }

    #[test]
    #[should_panic(expected = "price can't be calculated if any of the assets amount is zero")]
    fn test_get_price_asset_1_zero_amount_err() {
        let pool = ValenceXykPool {
            assets: vec![coin(100, "token_a"), coin(0, "token_b")],
            total_shares: "1000".to_string(),
            domain_specific_fields: BTreeMap::new(),
        };
        let query_msg = XykPoolQuery::GetPrice {};

        pool.query(to_json_binary(&query_msg).unwrap()).unwrap();
    }

    #[test]
    #[should_panic(expected = "price can be calculated iff xyk pool contains exactly 2 assets")]
    fn test_get_price_single_asset_err() {
        let pool = ValenceXykPool {
            assets: vec![coin(100, "token_a")],
            total_shares: "1000".to_string(),
            domain_specific_fields: BTreeMap::new(),
        };
        let query_msg = XykPoolQuery::GetPrice {};

        pool.query(to_json_binary(&query_msg).unwrap()).unwrap();
    }

    #[test]
    fn test_get_pool_asset_amount_happy() {
        let pool = ValenceXykPool {
            assets: vec![coin(100, "token_a"), coin(200, "token_b")],
            total_shares: "1000".to_string(),
            domain_specific_fields: BTreeMap::new(),
        };

        let query_msg = XykPoolQuery::GetPoolAssetAmount {
            target_denom: "token_a".to_string(),
        };
        let result = pool.query(to_json_binary(&query_msg).unwrap()).unwrap();
        assert_eq!(result, ValencePrimitive::Uint128(Uint128::new(100)));
    }

    #[test]
    #[should_panic(expected = "target coin not found")]
    fn test_get_pool_asset_amount_err() {
        let pool = ValenceXykPool {
            assets: vec![coin(100, "token_a"), coin(200, "token_b")],
            total_shares: "1000".to_string(),
            domain_specific_fields: BTreeMap::new(),
        };

        let query_msg = XykPoolQuery::GetPoolAssetAmount {
            target_denom: "non_existent".to_string(),
        };
        pool.query(to_json_binary(&query_msg).unwrap()).unwrap();
    }
}
