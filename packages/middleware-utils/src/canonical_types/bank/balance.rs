use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{from_json, to_json_binary, Binary, Coin, StdError, StdResult, Uint128};

use crate::type_registry::queries::ValenceTypeQuery;

#[cw_serde]
pub struct ValenceBankBalance {
    pub assets: Vec<Coin>,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum BankBalanceQuery {
    #[returns(Uint128)]
    GetDenomAmount { denom: String },
}

impl ValenceTypeQuery for ValenceBankBalance {
    fn query(&self, msg: Binary) -> StdResult<Binary> {
        let query_msg: BankBalanceQuery = from_json(&msg)?;
        match query_msg {
            BankBalanceQuery::GetDenomAmount { denom } => {
                for coin in &self.assets {
                    if coin.denom == denom {
                        return to_json_binary(&coin.amount);
                    }
                }
                Err(StdError::generic_err("denom not found"))
            }
        }
    }
}
