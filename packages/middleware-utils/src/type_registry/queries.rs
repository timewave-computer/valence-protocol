use cosmwasm_std::{Binary, StdResult};

use super::types::ValenceType;

pub trait ValenceTypeQuery {
    fn query(&self, msg: Binary) -> StdResult<Binary>;
}

impl ValenceTypeQuery for ValenceType {
    fn query(&self, query: Binary) -> StdResult<Binary> {
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
