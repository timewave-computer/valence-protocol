use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum MaxBtcExecuteMsg {
    Deposit { recipient: String },
}
