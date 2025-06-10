use std::collections::HashMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

#[cw_serde]
pub struct InstantiateMsg {
    pub denom_ratios: HashMap<String, Decimal>,
}

#[cw_serde]
pub enum ExecuteMsg {}
