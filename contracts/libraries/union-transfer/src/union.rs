use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, Uint64};

#[cw_serde]
pub enum ExecuteMsg {
    Send {
        channel_id: u64,
        timeout_height: Uint64,
        timeout_timestamp: Uint64,
        salt: HexBinary,
        instruction: HexBinary,
    },
}
