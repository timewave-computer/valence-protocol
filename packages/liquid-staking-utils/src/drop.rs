use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum LiquidStakerExecuteMsg {
    Bond {
        receiver: Option<String>,
        r#ref: Option<String>,
    },
}
