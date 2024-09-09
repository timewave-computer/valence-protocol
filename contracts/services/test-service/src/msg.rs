use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    WillError { error: String },
    WillSucceed { execution_id: Option<u64> },
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
pub enum MigrateMsg {
    Migrate { new_condition: bool },
}
