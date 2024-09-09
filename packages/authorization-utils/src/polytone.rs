use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, CosmosMsg, Empty, Uint64};

#[cw_serde]
pub enum PolytoneExecuteMsg {
    Execute {
        msgs: Vec<CosmosMsg<Empty>>,
        callback: Option<CallbackRequest>,
        timeout_seconds: Uint64,
    },
}

#[cw_serde]
pub struct CallbackRequest {
    pub receiver: String,
    pub msg: Binary,
}
