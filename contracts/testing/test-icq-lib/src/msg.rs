use cosmwasm_schema::{cw_serde, QueryResponses};
use neutron_sdk::{
    bindings::types::InterchainQueryResult, interchain_queries::v047::queries::BalanceResponse,
};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    RegisterBalancesQuery {
        connection_id: String,
        update_period: u64,
        addr: String,
        denoms: Vec<String>,
    },
    RegisterKeyValueQuery {
        connection_id: String,
        update_period: u64,
        path: String,
        key: Vec<u8>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(BalanceResponse)]
    Balance { query_id: u64 },
    #[returns(Vec<(String, String)>)]
    Catchall {},
    #[returns(InterchainQueryResult)]
    RawIcqResult { id: u64 },
}

#[cw_serde]
pub enum MigrateMsg {}
