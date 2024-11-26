use cosmwasm_schema::{cw_serde, QueryResponses};
use neutron_sdk::interchain_queries::v047::queries::BalanceResponse;

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
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(BalanceResponse)]
    Balance { query_id: u64 },
}

#[cw_serde]
pub enum MigrateMsg {}
