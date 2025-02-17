use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{from_json, Binary, Coin, StdError, StdResult};

use crate::type_registry::queries::{ValencePrimitive, ValenceTypeQuery};

#[cw_serde]
pub struct ValenceBankBalance {
    pub assets: Vec<Coin>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum BankBalanceQuery {
    // IMPORTANT: if you add new variants here that return one of the following response types:
    // - String
    // - Uint64
    // - Uint256
    // make sure to extend the unit tests under contracts/middleware/asserter/src/testing
    // to cover that response type assertions.
    #[returns(ValencePrimitive)]
    GetDenomAmount { denom: String },
}

impl ValenceTypeQuery for ValenceBankBalance {
    fn query(&self, msg: Binary) -> StdResult<ValencePrimitive> {
        let query_msg: BankBalanceQuery = from_json(&msg)?;
        match query_msg {
            BankBalanceQuery::GetDenomAmount { denom } => {
                for coin in &self.assets {
                    if coin.denom == denom {
                        return Ok(ValencePrimitive::Uint128(coin.amount));
                    }
                }
                Err(StdError::generic_err("denom not found"))
            }
        }
    }
}
