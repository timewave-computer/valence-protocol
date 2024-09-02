use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    WillError { error: String },
    WillSucceed {},
    WillSucceedEveryFiveTimes {},
    SendCallback { to: String, callback: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(u64)]
    Counter {},
}

#[cw_serde]
pub struct MigrateMsg {
    pub new_counter: u64,
}
