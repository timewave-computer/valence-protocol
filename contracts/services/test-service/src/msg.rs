use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    WillError { error: String },
    WillSucceed {},
    WillSucceedIfTrue {},
    SetCondition { condition: bool },
    SendCallback { to: String, callback: Binary },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    Condition {},
}

#[cw_serde]
pub struct MigrateMsg {
    pub new_condition: bool,
}
