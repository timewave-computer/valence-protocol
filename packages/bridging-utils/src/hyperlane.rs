use cosmwasm_schema::cw_serde;
use cosmwasm_std::HexBinary;

#[cw_serde]
pub enum HyperlaneExecuteMsg {
    Dispatch(DispatchMsg),
}

#[cw_serde]
pub struct DispatchMsg {
    pub dest_domain: u32,
    pub recipient_addr: HexBinary,
    pub msg_body: HexBinary,
    pub hook: Option<String>,
    pub metadata: Option<HexBinary>,
}
