use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, Decimal, StdResult, Uint128, Uint256, Uint64};

use super::types::ValenceType;

/// supported evaluation types. both assertion values must be of this type
/// in order to evaluate the condition.
#[cw_serde]
pub enum ValencePrimitive {
    Decimal(Decimal),
    Uint64(Uint64),
    Uint128(Uint128),
    Uint256(Uint256),
    String(String),
}

pub trait ValenceTypeQuery {
    fn query(&self, msg: Binary) -> StdResult<ValencePrimitive>;
}

impl ValenceTypeQuery for ValenceType {
    fn query(&self, query: Binary) -> StdResult<ValencePrimitive> {
        // IMPORTANT: if you add new variants here that are capable of querying with
        // response values of:
        // - String
        // - Uint64
        // - Uint256
        // make sure to extend the unit tests under contracts/middleware/asserter/src/testing
        // with the new variant and the expected response type.
        let queryable: &dyn ValenceTypeQuery = match self {
            ValenceType::XykPool(var) => var,
            ValenceType::BankBalance(var) => var,
        };
        queryable.query(query)
    }
}
