use cosmwasm_std::{Binary, StdResult};

use super::types::ValenceType;

pub trait ValenceTypeQuery {
    fn query(&self, msg: Binary) -> StdResult<Binary>;
}

impl ValenceTypeQuery for ValenceType {
    fn query(&self, query: Binary) -> StdResult<Binary> {
        // TODO: move this to some macro or something to avoid manual matching
        // dynamically dispatch to the correct implementation
        let queryable: &dyn ValenceTypeQuery = match self {
            ValenceType::XykPool(var) => var,
            ValenceType::BankBalance(var) => var,
        };
        queryable.query(query)
    }
}
