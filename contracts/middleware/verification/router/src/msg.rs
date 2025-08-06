use std::collections::HashMap;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub initial_routes: HashMap<String, String>,
    pub owner: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    AddRoute { name: String, address: String },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    GetRoute { name: String },
    #[returns(Vec<Addr>)]
    GetRoutes {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(bool)]
    Verify {
        route: String,
        vk: Binary,
        inputs: Binary,
        proof: Binary,
        payload: Binary,
    },
}
