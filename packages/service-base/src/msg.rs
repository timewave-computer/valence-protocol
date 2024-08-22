use cosmwasm_schema::cw_serde;
use cw_ownable::cw_ownable_execute;

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg<T, U> {
    ProcessAction(T),
    UpdateConfig { new_config: U },
    UpdateProcessor { processor: String },
}
